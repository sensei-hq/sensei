use super::*;

// ── Extensions tests ───────────────────────────────────────────────

#[tokio::test]
async fn extension_create_and_list() {
    let s = pg_store().await;
    let name = format!("_test:ext_{}", uuid::Uuid::new_v4());
    let id = s
        .create_extension("skill", &name, Some("test skill"), Some("# content"), "global", "local")
        .await
        .unwrap();
    let skills = s.list_extensions_by_kind("skill").await.unwrap();
    assert!(skills.iter().any(|e| e["name"] == name));
    s.delete_extension(&id).await.unwrap();
}

#[tokio::test]
async fn extension_historize_trigger() {
    let s = pg_store().await;
    let name = format!("_test:ext_hist_{}", uuid::Uuid::new_v4());
    let id = s.create_extension("skill", &name, Some("v1"), None, "global", "local").await.unwrap();
    s.update_extension(&id, Some("v2"), None).await.unwrap();
    let history = s.get_extension_history(&id).await.unwrap();
    assert!(history.len() >= 2, "historize trigger should create INSERT + UPDATE entries");
    s.delete_extension(&id).await.unwrap();
}

// ── Benchmark Reports tests ──────────────────────────────────────

#[tokio::test]
async fn benchmark_create_and_list() {
    let s = pg_store().await;
    let id = s
        .create_benchmark_report(
            None,
            "_test:bench",
            "strategy_a",
            Some(95.5),
            Some(1000),
            Some(5000),
        )
        .await
        .unwrap();
    let reports = s.list_benchmark_reports().await.unwrap();
    assert!(reports.iter().any(|r| r["run_name"] == "_test:bench"));
    sqlx_core::query::query("DELETE FROM sensei.benchmark_reports WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Views tests ──────────────────────────────────────────────────

#[tokio::test]
async fn repositories_view() {
    let s = pg_store().await;
    // list_repositories returns git+subtree folders
    let repos = s.list_repositories().await.unwrap();
    // Just verify it doesn't error — content depends on seeded data
    // Just verify the query succeeds — content depends on seeded data
    let _ = repos;
}

// ── Reasoning Traces tests ───────────────────────────────────────

#[tokio::test]
async fn reasoning_trace_insert_and_get() {
    let s = pg_store().await;
    let pid = s.create_workspace("_test:rt_proj", None, None).await.unwrap();
    let tid = s
        .insert_reasoning_trace(
            Some(&pid),
            "pattern_emerging",
            &serde_json::json!({}),
            &["gemma4:27b".into()],
            &serde_json::json!([{"model":"gemma4","role":"proposer","content":"analyze"}]),
            &serde_json::json!({"conclusion":"adopt adapter pattern","confidence":0.9}),
        )
        .await
        .unwrap();
    let traces = s.get_reasoning_traces_by_workspace(&pid).await.unwrap();
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0]["consensus"]["confidence"], 0.9);
    assert_eq!(traces[0]["trigger_event"], "pattern_emerging");
    sqlx_core::query::query("DELETE FROM inference.reasoning_traces WHERE id = $1")
        .bind(tid)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_workspace(&pid).await.unwrap();
}

// ── Services tests ───────────────────────────────────────────────

#[tokio::test]
async fn service_upsert_and_list() {
    let s = pg_store().await;
    let name = format!("_test:svc_{}", uuid::Uuid::new_v4());
    let id = s
        .upsert_service(
            &name,
            "Test MCP",
            "data",
            "mcp",
            &serde_json::json!({"url":"http://localhost"}),
        )
        .await
        .unwrap();
    let svcs = s.list_services().await.unwrap();
    assert!(svcs.iter().any(|sv| sv["name"] == name));
    s.delete_service(&name).await.unwrap();
    let _ = id;
}

// ── Snapshots tests ──────────────────────────────────────────────

#[tokio::test]
async fn snapshot_create_and_get_latest() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("snap_{}", uuid::Uuid::new_v4())).await;
    let sid = s.create_session(&fid, "snapshot test", None).await.unwrap();
    s.create_snapshot(&sid, &fid, "manual", "Step 1 done", Some("Do step 2"), &["Step 1".into()])
        .await
        .unwrap();
    s.create_snapshot(
        &sid,
        &fid,
        "checkpoint",
        "Step 2 done",
        None,
        &["Step 1".into(), "Step 2".into()],
    )
    .await
    .unwrap();
    let latest = s.get_latest_snapshot(&sid).await.unwrap().unwrap();
    assert_eq!(latest["progress_summary"], "Step 2 done");
    assert_eq!(latest["kind"], "checkpoint");
    assert_eq!(latest["completed_steps"].as_array().unwrap().len(), 2);
}

// ── Workflow State tests ────────────────────────────────────────────

#[tokio::test]
async fn wf_upsert_and_get() {
    let s = pg_store().await;
    let p = "_test:wf:upsert";
    s.delete_workflow_state(p).await.unwrap();
    assert!(s.get_workflow_state(p).await.unwrap().is_none());
    s.upsert_workflow_state(p, Some("ideate"), None, None, None, None, None).await.unwrap();
    let state = s.get_workflow_state(p).await.unwrap().unwrap();
    assert_eq!(state["active_phase"], "ideate");
    assert!(state["active_task"].is_null());
    s.delete_workflow_state(p).await.unwrap();
}

#[tokio::test]
async fn wf_partial_update_preserves() {
    let s = pg_store().await;
    let p = "_test:wf:partial";
    s.delete_workflow_state(p).await.unwrap();
    s.upsert_workflow_state(
        p,
        Some("build"),
        Some("plan.md"),
        Some("task 1"),
        Some(42),
        None,
        Some("hash123"),
    )
    .await
    .unwrap();
    s.upsert_workflow_state(p, Some("validate"), None, None, None, None, None).await.unwrap();
    let state = s.get_workflow_state(p).await.unwrap().unwrap();
    assert_eq!(state["active_phase"], "validate");
    assert_eq!(state["active_plan"], "plan.md");
    assert_eq!(state["active_task"], "task 1");
    assert_eq!(state["active_issue"], 42);
    s.delete_workflow_state(p).await.unwrap();
}

#[tokio::test]
async fn wf_nonexistent_returns_none() {
    let s = pg_store().await;
    assert!(s.get_workflow_state("_test:wf:none").await.unwrap().is_none());
}

#[tokio::test]
async fn wf_delete() {
    let s = pg_store().await;
    let p = "_test:wf:delete";
    s.upsert_workflow_state(p, Some("ideate"), None, None, None, None, None).await.unwrap();
    s.delete_workflow_state(p).await.unwrap();
    assert!(s.get_workflow_state(p).await.unwrap().is_none());
}

// ── Tags tests ────────────────────────────────────────────────────

#[tokio::test]
async fn tag_add_and_list() {
    let s = pg_store().await;
    let tag = "_test:tag_add:rust";
    s.add_tag(tag, Some("stack")).await.unwrap();
    let tags = s.list_tags().await.unwrap();
    assert!(tags.iter().any(|(t, c)| t == tag && c.as_deref() == Some("stack")));
    s.remove_tag(tag).await.unwrap();
}

#[tokio::test]
async fn tag_add_without_category() {
    let s = pg_store().await;
    let tag = "_test:tag_nocat:misc";
    s.add_tag(tag, None).await.unwrap();
    let tags = s.list_tags().await.unwrap();
    assert!(tags.iter().any(|(t, c)| t == tag && c.is_none()));
    s.remove_tag(tag).await.unwrap();
}

#[tokio::test]
async fn tag_add_duplicate_is_upsert() {
    let s = pg_store().await;
    let tag = "_test:tag_dup:ts";
    s.add_tag(tag, Some("stack")).await.unwrap();
    s.add_tag(tag, Some("language")).await.unwrap(); // update category
    let tags = s.list_tags().await.unwrap();
    let found: Vec<_> = tags.iter().filter(|(t, _)| t == tag).collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1.as_deref(), Some("language"));
    s.remove_tag(tag).await.unwrap();
}

#[tokio::test]
async fn tag_remove() {
    let s = pg_store().await;
    let tag = "_test:tag_rm:go";
    s.add_tag(tag, Some("stack")).await.unwrap();
    s.remove_tag(tag).await.unwrap();
    let tags = s.list_tags().await.unwrap();
    assert!(!tags.iter().any(|(t, _)| t == tag));
}

#[tokio::test]
async fn tag_remove_nonexistent_is_noop() {
    let s = pg_store().await;
    s.remove_tag("_test:tag_rm_noop:xyz").await.unwrap();
}

#[tokio::test]
async fn tag_list_by_category() {
    let s = pg_store().await;
    let t1 = "_test:tag_cat:rust";
    let t2 = "_test:tag_cat:ts";
    let t3 = "_test:tag_cat:active";
    s.add_tag(t1, Some("stack")).await.unwrap();
    s.add_tag(t2, Some("stack")).await.unwrap();
    s.add_tag(t3, Some("status")).await.unwrap();
    let stack_tags = s.list_tags_by_category("stack").await.unwrap();
    assert!(stack_tags.contains(&t1.to_string()));
    assert!(stack_tags.contains(&t2.to_string()));
    assert!(!stack_tags.contains(&t3.to_string()));
    s.remove_tag(t1).await.unwrap();
    s.remove_tag(t2).await.unwrap();
    s.remove_tag(t3).await.unwrap();
}

// ── Schema tests ─────────────────────────────────────────────────

#[tokio::test]
async fn memories_table_exists() {
    let store = PgStore::connect_test().await.unwrap();
    let row: (bool,) = query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema = 'sensei' AND table_name = 'memories')"
        )
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert!(row.0, "sensei.memories table must exist — run `dbd apply` first");
}

// ── Knowledge Sources tests ───────────────────────────────────────

#[tokio::test]
async fn knowledge_source_crud_roundtrip() {
    let pg = PgStore::test_store().await;
    let id = pg
        .create_knowledge_source(&NewKnowledgeSource {
            kind: "hive_mind".into(),
            name: "Org Dōjō".into(),
            url: "https://dojo.example".into(),
            namespace_id: None,
            direction: "both".into(),
        })
        .await
        .unwrap();

    let all = pg.list_knowledge_sources().await.unwrap();
    assert!(all.iter().any(|s| s.id == id && s.last_seq == 0 && s.enabled));

    pg.set_source_cursor(&id, 42).await.unwrap();
    let one = pg.get_knowledge_source(&id).await.unwrap().unwrap();
    assert_eq!(one.last_seq, 42);
    assert_eq!(one.direction, "both");

    assert!(pg.delete_knowledge_source(&id).await.unwrap());
    assert!(pg.get_knowledge_source(&id).await.unwrap().is_none());
}
