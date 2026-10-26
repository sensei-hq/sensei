use super::*;

// ── Memories tests ─────────────────────────────────────────────────

#[tokio::test]
async fn memory_create_and_get() {
    let s = pg_store().await;
    let id = s
        .create_memory(&MemoryRow {
            workspace_id: None,
            scope: "global",
            scope_filter: None,
            mem_type: "decision",
            title: "_test:mem_create",
            content: "Always use TDD",
            impact: Some("Bugs ship to prod"),
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let m = s.get_memory(&id).await.unwrap().unwrap();
    assert_eq!(m["title"], "_test:mem_create");
    assert_eq!(m["scope"], "global");
    assert_eq!(m["strength"], 1.0);
    assert_eq!(m["status"], "active");
    // cleanup via historize trigger test
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

/// #307 R3b: a workspace-scoped memory says `workspace` end to end — the API
/// that writes it, the database that stores it, and every read handed back.
#[tokio::test]
async fn a_workspace_scope_is_stored_and_read_back_as_workspace() {
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:mem_scope-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let id = s
        .create_memory(&MemoryRow {
            workspace_id: Some(&pid),
            scope: "workspace",
            scope_filter: None,
            mem_type: "decision",
            title: "_test:mem_scope",
            content: "c",
            impact: None,
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let (stored,): (String,) =
        sqlx_core::query_as::query_as("SELECT scope::text FROM sensei.memories WHERE id = $1")
            .bind(id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(stored, "workspace", "the database says workspace");
    assert_eq!(s.get_memory(&id).await.unwrap().unwrap()["scope"], "workspace");
    let listed = s.list_memories(Some(pid), None, Some("workspace"), 10).await.unwrap();
    assert_eq!(listed.len(), 1, "a `workspace` filter finds the workspace-scoped row");
    assert_eq!(listed[0]["scope"], "workspace");
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn create_memory_persists_spine_slot_and_feature() {
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:mem_slot-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let id = s
        .create_memory(&MemoryRow {
            workspace_id: Some(&pid),
            scope: "workspace",
            scope_filter: None,
            mem_type: "decision",
            title: "t",
            content: "c",
            impact: None,
            session_id: None,
            spine_slot: Some("decisions"),
            feature: Some("auth"),
        })
        .await
        .unwrap();
    let row: (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT spine_slot::text, feature FROM sensei.memories WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(row, (Some("decisions".into()), Some("auth".into())));
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn memory_reinforce() {
    let s = pg_store().await;
    let id = s
        .create_memory(&MemoryRow {
            workspace_id: None,
            scope: "global",
            scope_filter: None,
            mem_type: "pattern",
            title: "_test:mem_reinforce",
            content: "rule",
            impact: None,
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    s.reinforce_memory(&id, 1.0).await.unwrap();
    s.reinforce_memory(&id, 1.0).await.unwrap();
    let m = s.get_memory(&id).await.unwrap().unwrap();
    assert_eq!(m["strength"], 3.0); // 1.0 + 1.0 + 1.0
    // Cap at 5.0
    s.reinforce_memory(&id, 10.0).await.unwrap();
    let m = s.get_memory(&id).await.unwrap().unwrap();
    assert_eq!(m["strength"], 5.0);
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn memory_archive() {
    let s = pg_store().await;
    let id = s
        .create_memory(&MemoryRow {
            workspace_id: None,
            scope: "global",
            scope_filter: None,
            mem_type: "question",
            title: "_test:mem_archive",
            content: "open q",
            impact: None,
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    s.archive_memory(&id).await.unwrap();
    let m = s.get_memory(&id).await.unwrap().unwrap();
    assert_eq!(m["status"], "archived");
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn memory_list_active() {
    let s = pg_store().await;
    let id1 = s
        .create_memory(&MemoryRow {
            workspace_id: None,
            scope: "global",
            scope_filter: None,
            mem_type: "decision",
            title: "_test:mem_list_a",
            content: "rule a",
            impact: None,
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let id2 = s
        .create_memory(&MemoryRow {
            workspace_id: None,
            scope: "global",
            scope_filter: None,
            mem_type: "decision",
            title: "_test:mem_list_b",
            content: "rule b",
            impact: None,
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let active = s.list_active_memories(None, Some("global")).await.unwrap();
    assert!(active.iter().any(|m| m["title"] == "_test:mem_list_a"));
    assert!(active.iter().any(|m| m["title"] == "_test:mem_list_b"));
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
        .bind(&[id1, id2][..])
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Memory Examples tests ────────────────────────────────────────

#[tokio::test]
async fn memory_example_add_and_list() {
    let s = pg_store().await;
    let mid = s
        .create_memory(&MemoryRow {
            workspace_id: None,
            scope: "global",
            scope_filter: None,
            mem_type: "pattern",
            title: "_test:mem_ex",
            content: "rule",
            impact: None,
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    s.add_memory_example(&mid, "fn:auth_handler", true, Some("canonical auth")).await.unwrap();
    s.add_memory_example(&mid, "fn:inline_auth", false, Some("avoid inline")).await.unwrap();
    let examples = s.list_memory_examples(&mid).await.unwrap();
    assert_eq!(examples.len(), 2);
    assert!(examples.iter().any(|e| e["is_good"] == true));
    assert!(examples.iter().any(|e| e["is_good"] == false));
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(mid)
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Memory Evidence tests ────────────────────────────────────────

#[tokio::test]
async fn memory_evidence_add_and_list() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("mem_ev_{}", uuid::Uuid::new_v4())).await;
    let sid = s.create_session(&fid, "test", None).await.unwrap();
    let mid = s
        .create_memory(&MemoryRow {
            workspace_id: None,
            scope: "global",
            scope_filter: None,
            mem_type: "decision",
            title: "_test:mem_ev",
            content: "rule",
            impact: None,
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    s.add_memory_evidence(&mid, Some(&sid), Some("user corrected twice")).await.unwrap();
    // A save-time source note carries no session_id (nullable).
    s.add_memory_evidence(&mid, None, Some("crates/x.rs:42")).await.unwrap();
    let evidence = s.list_memory_evidence(&mid).await.unwrap();
    assert_eq!(evidence.len(), 2);
    assert!(
        evidence.iter().any(|e| e["session_id"].is_null() && e["note"] == "crates/x.rs:42"),
        "the session-less source note round-trips with a null session_id"
    );
    assert_eq!(evidence[0]["note"], "user corrected twice");
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(mid)
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Memory Links tests ───────────────────────────────────────────

#[tokio::test]
async fn memory_links_parent_child() {
    let s = pg_store().await;
    let parent = s
        .create_memory(&MemoryRow {
            workspace_id: None,
            scope: "global",
            scope_filter: None,
            mem_type: "decision",
            title: "_test:mem_parent",
            content: "combined",
            impact: None,
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let child1 = s
        .create_memory(&MemoryRow {
            workspace_id: None,
            scope: "global",
            scope_filter: None,
            mem_type: "decision",
            title: "_test:mem_child1",
            content: "original 1",
            impact: None,
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let child2 = s
        .create_memory(&MemoryRow {
            workspace_id: None,
            scope: "global",
            scope_filter: None,
            mem_type: "decision",
            title: "_test:mem_child2",
            content: "original 2",
            impact: None,
            session_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    s.link_memories(&parent, &child1).await.unwrap();
    s.link_memories(&parent, &child2).await.unwrap();
    let children = s.get_memory_children(&parent).await.unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(s.get_memory_parent(&child1).await.unwrap(), Some(parent));
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
        .bind(&[parent, child1, child2][..])
        .execute(s.pool())
        .await
        .unwrap();
}

// ── Recommendations tests ────────────────────────────────────────

#[tokio::test]
async fn recommendation_lifecycle() {
    let s = pg_store().await;
    let pid = s.create_workspace("_test:rec_proj", None, None).await.unwrap();
    let rid = s
        .create_recommendation(&pid, "_test:rec", "reduces corrections", "promote_pattern", "high")
        .await
        .unwrap();
    s.accept_recommendation(&rid).await.unwrap();
    s.measure_recommendation(&rid, "positive").await.unwrap();
    let recs = s.list_recommendations(&pid).await.unwrap();
    let r = recs.iter().find(|r| r["title"] == "_test:rec").unwrap();
    assert_eq!(r["status"], "accepted");
    assert_eq!(r["verdict"], "positive");
    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_workspace(&pid).await.unwrap();
}

// Gap 1 fix — the reject terminal is `dismissed` in the enum (not
// `rejected`), and both accept + reject must only fire from `pending`.
// Locks the enum-value contract so a future rename can't silently
// break the UI action buttons.
#[tokio::test]
async fn recommendation_reject_writes_dismissed_and_guards_at_pending() {
    let s = pg_store().await;
    let pid = s.create_workspace("_test:rec_reject_proj", None, None).await.unwrap();
    let rid =
        s.create_recommendation(&pid, "_test:rej", "why", "revise_rule", "low").await.unwrap();

    s.reject_recommendation(&rid).await.unwrap();
    let recs = s.list_recommendations(&pid).await.unwrap();
    let r = recs.iter().find(|r| r["title"] == "_test:rej").unwrap();
    assert_eq!(r["status"], "dismissed", "reject writes the `dismissed` enum terminal");

    // Second reject on the same rec is a no-op guarded at `pending`;
    // pg_store must return an error rather than clobber the decision.
    let err = s.reject_recommendation(&rid).await.expect_err("guard fires on already-decided");
    assert!(
        err.contains("already decided") || err.contains("not found"),
        "guard error text: {err}"
    );

    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_workspace(&pid).await.unwrap();
}

// ── P-A: accept → materialize a governance RULE (spec 2026-08-20) ──
#[tokio::test]
async fn accept_as_rule_materializes_a_memory_and_records_provenance() {
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:mat-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    // A revise_rule rec — the canonical rule-class case ("why are you using regex").
    let rid = s
        .create_recommendation(
            &pid,
            "No bare regex in the parser",
            "regex is fragile here; use the typed lexer",
            "revise_rule",
            "high",
        )
        .await
        .unwrap();

    // Accept + materialize at gov_scope=general (namespace None) enforcement=required.
    let mat = s
        .accept_recommendation_as_rule(
            &rid,
            &crate::db::pg_store::RuleAcceptance {
                namespace_id: None,
                enforcement: Some("required"),
                gov_scope: "general",
                title_override: None,
                body_override: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(mat["kind"], "rule");
    assert_eq!(mat["scope"], "general");
    assert_eq!(mat["enforcement"], "required");
    let mem_id = uuid::Uuid::parse_str(mat["memory_id"].as_str().unwrap()).unwrap();

    // The memory is LIVE (active) with the rec's content + chosen enforcement + origin.
    let (title, content, status, enforcement, mtype, origin): (String, String, String, String, String, String) =
            sqlx_core::query_as::query_as(
                "SELECT title, content, status::text, enforcement::text, type::text, origin::text FROM sensei.memories WHERE id=$1"
            ).bind(mem_id).fetch_one(s.pool()).await.unwrap();
    assert_eq!(title, "No bare regex in the parser");
    assert_eq!(
        content, "regex is fragile here; use the typed lexer",
        "body defaults to the rec's why"
    );
    assert_eq!(status, "active", "an accepted rule is live immediately");
    assert_eq!(enforcement, "required");
    assert_eq!(mtype, "convention");
    assert_eq!(origin, "authored");

    // The rec is accepted + carries the materialized_ref provenance.
    let (rstatus, ref_kind, ref_mem): (String, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT status::text, materialized_ref->>'kind', materialized_ref->>'memory_id' FROM inference.recommendations WHERE id=$1"
        ).bind(rid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(rstatus, "accepted");
    assert_eq!(ref_kind.as_deref(), Some("rule"));
    assert_eq!(
        ref_mem.as_deref(),
        Some(mem_id.to_string().as_str()),
        "provenance links the exact memory"
    );

    // Idempotent guard: a second materialize on the now-accepted rec errors (no double write).
    let err = s
        .accept_recommendation_as_rule(
            &rid,
            &crate::db::pg_store::RuleAcceptance {
                namespace_id: None,
                enforcement: None,
                gov_scope: "general",
                title_override: None,
                body_override: None,
            },
        )
        .await
        .expect_err("guarded at pending");
    assert!(err.contains("already decided") || err.contains("not found"), "guard: {err}");

    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id=$1")
        .bind(mem_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id=$1")
        .bind(rid)
        .execute(s.pool())
        .await
        .ok();
    s.delete_workspace(&pid).await.unwrap();
}

#[tokio::test]
async fn accept_as_rule_rejects_non_rule_action_and_title_body_override() {
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:mat2-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();

    // Non-rule action (audit_stale) → error, no memory written (mis-route surfaced).
    let audit = s
        .create_recommendation(&pid, "_test:audit", "high rework file", "audit_stale", "low")
        .await
        .unwrap();
    let err = s
        .accept_recommendation_as_rule(
            &audit,
            &crate::db::pg_store::RuleAcceptance {
                namespace_id: None,
                enforcement: None,
                gov_scope: "general",
                title_override: None,
                body_override: None,
            },
        )
        .await
        .expect_err("non-rule rejected");
    assert!(err.contains("not rule-class"), "got: {err}");

    // enrich_memory with title + body overrides (the review-before-apply edit path).
    let rid = s
        .create_recommendation(&pid, "orig title", "orig why", "enrich_memory", "medium")
        .await
        .unwrap();
    let mat = s
        .accept_recommendation_as_rule(
            &rid,
            &crate::db::pg_store::RuleAcceptance {
                namespace_id: None,
                enforcement: Some("advisory"),
                gov_scope: "general",
                title_override: Some("edited title"),
                body_override: Some("edited body"),
            },
        )
        .await
        .unwrap();
    let mem_id = uuid::Uuid::parse_str(mat["memory_id"].as_str().unwrap()).unwrap();
    let (title, content): (String, String) =
        sqlx_core::query_as::query_as("SELECT title, content FROM sensei.memories WHERE id=$1")
            .bind(mem_id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(title, "edited title", "title override wins");
    assert_eq!(content, "edited body", "body override wins");

    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id=$1")
        .bind(mem_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE workspace_id=$1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
    s.delete_workspace(&pid).await.unwrap();
}

#[tokio::test]
async fn begin_file_materialization_flips_and_returns_prompt_seed() {
    // P-B store contract: the guarded flip returns the (action_type, title, why,
    // prompt) seed a file materializer renders, and set_recommendation_materialized
    // records the file provenance. Second flip is guarded (no double write).
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:pbmat-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let rid = s.create_recommendation_full(&RecommendationRow {
        workspace_id: &pid,
        title: "Establish DBD Guardian Agent",
        why: "cross-layer churn needs a review agent",
        impact: None,
        action_type: "create_agent",
        urgency: "high",
        based_on: &serde_json::json!({}),
        reasoning_trace_id: None,
        prompt: Some("You are an Architectural Review Agent for dbd. Before any code is accepted, check module boundaries."),
    }).await.unwrap();

    let (action_type, title, why, prompt) =
        s.begin_file_materialization(&rid).await.unwrap().expect("pending → seed");
    assert_eq!(action_type, "create_agent");
    assert_eq!(title, "Establish DBD Guardian Agent");
    assert_eq!(why, "cross-layer churn needs a review agent");
    assert!(
        prompt.as_deref().unwrap().starts_with("You are an Architectural Review Agent"),
        "prompt seed returned"
    );

    // Record the file provenance (what the handler does after writing the file).
    let mref = serde_json::json!({ "kind": "agent", "file_path": ".claude/agents/establish-dbd-guardian-agent.md" });
    s.set_recommendation_materialized(&rid, &mref).await.unwrap();
    let (status, kind, fp): (String, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT status::text, materialized_ref->>'kind', materialized_ref->>'file_path' FROM inference.recommendations WHERE id=$1"
        ).bind(rid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(status, "accepted");
    assert_eq!(kind.as_deref(), Some("agent"));
    assert_eq!(fp.as_deref(), Some(".claude/agents/establish-dbd-guardian-agent.md"));

    // Guarded: a second flip on the accepted rec returns None (no re-materialize).
    assert!(s.begin_file_materialization(&rid).await.unwrap().is_none(), "guarded at pending");

    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id=$1")
        .bind(rid)
        .execute(s.pool())
        .await
        .ok();
    s.delete_workspace(&pid).await.unwrap();
}

// ── Accept-driven pattern promotion ──────────────────────────────
// Accepting a `promote_pattern` rec advances its source pattern's
// lifecycle to `rule` (the read path renders it `adopted`). The action
// is store-owned so it stays single-call-site + unit-testable.

/// Seed a (workspace, folder, pattern, promote_pattern rec) fixture. The rec's
/// `based_on.patterns[0]` cites the pattern (unless `cite_pattern` is false,
/// exercising the defensive no-op path). Returns (proj, folder, pattern, rec).
async fn seed_promote_fixture(
    s: &PgStore,
    suffix: &str,
    action_type: &str,
    cite_pattern: bool,
) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let (proj_id, fid) = create_test_workspace_and_folder(s, suffix).await;
    let pat_id = s
        .upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid),
            name: "_test:rule-candidates",
            is_anti: false,
            confidence: None,
            instances: &serde_json::json!([]),
        })
        .await
        .unwrap();
    // suggested is the seeded lifecycle default; assert the precondition so a
    // schema default change can't make the promotion test vacuously pass.
    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let seeded = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(seeded["lifecycle"], "suggested", "pattern starts at suggested");

    let based_on = if cite_pattern {
        serde_json::json!({ "patterns": [pat_id] })
    } else {
        serde_json::json!({ "patterns": [] })
    };
    let rid = s
        .create_recommendation_full(&RecommendationRow {
            workspace_id: &proj_id,
            title: "_test:promote",
            why: "why",
            impact: None,
            action_type,
            urgency: "medium",
            based_on: &based_on,
            reasoning_trace_id: None,
            prompt: None,
        })
        .await
        .unwrap();
    (proj_id, fid, pat_id, rid)
}

async fn cleanup_promote_fixture(
    s: &PgStore,
    proj_id: &uuid::Uuid,
    pat_id: &uuid::Uuid,
    rid: &uuid::Uuid,
) {
    sqlx_core::query::query("DELETE FROM inference.recommendations WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(pat_id)
        .execute(s.pool())
        .await
        .unwrap();
    s.delete_workspace(proj_id).await.unwrap();
}

/// Pure extractor: only a well-formed `patterns[0]` uuid comes back; a
/// missing key, empty array, or non-uuid is `None` (the no-op signal).
#[test]
fn based_on_first_pattern_parses_uuid_and_defends() {
    let id = uuid::Uuid::new_v4();
    let good = serde_json::json!({ "patterns": [id] }).to_string();
    assert_eq!(PgStore::based_on_first_pattern(&good), Some(id));
    assert_eq!(PgStore::based_on_first_pattern("{}"), None);
    assert_eq!(PgStore::based_on_first_pattern(r#"{"patterns":[]}"#), None);
    assert_eq!(PgStore::based_on_first_pattern(r#"{"patterns":["not-a-uuid"]}"#), None);
    assert_eq!(PgStore::based_on_first_pattern("not json"), None);
}

/// (1) Accepting a promote_pattern rec advances the cited pattern to `rule`.
#[tokio::test]
async fn accept_promote_pattern_advances_lifecycle_to_rule() {
    let s = pg_store().await;
    let suffix = format!("accept_promote_{}", uuid::Uuid::new_v4());
    let (proj_id, fid, pat_id, rid) =
        seed_promote_fixture(&s, &suffix, "promote_pattern", true).await;

    s.accept_recommendation(&rid).await.unwrap();

    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(
        p["lifecycle"], "rule",
        "accepting a promote_pattern rec advances the pattern to rule"
    );

    cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
}

/// (2) A non-promote action (e.g. write_skill) leaves the pattern untouched.
#[tokio::test]
async fn accept_non_promote_leaves_pattern_untouched() {
    let s = pg_store().await;
    let suffix = format!("accept_writeskill_{}", uuid::Uuid::new_v4());
    let (proj_id, fid, pat_id, rid) = seed_promote_fixture(&s, &suffix, "write_skill", true).await;

    s.accept_recommendation(&rid).await.unwrap();

    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(p["lifecycle"], "suggested", "a non-promote action must not advance the pattern");

    cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
}

/// (3) A promote_pattern rec with no cited pattern accepts as a no-op —
/// returns Ok, the pattern is unchanged, and nothing panics.
#[tokio::test]
async fn accept_promote_pattern_without_provenance_is_noop() {
    let s = pg_store().await;
    let suffix = format!("accept_noprov_{}", uuid::Uuid::new_v4());
    let (proj_id, fid, pat_id, rid) =
        seed_promote_fixture(&s, &suffix, "promote_pattern", false).await;

    s.accept_recommendation(&rid).await.expect("empty provenance accepts without error");

    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(p["lifecycle"], "suggested", "no cited pattern → nothing to promote");

    cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
}

/// (4) The pending-guard holds: a second accept errors, and the promotion
/// fired exactly once (the pattern is still `rule`, not re-touched into error).
#[tokio::test]
async fn accept_promote_pattern_guard_fires_promotion_once() {
    let s = pg_store().await;
    let suffix = format!("accept_guard_{}", uuid::Uuid::new_v4());
    let (proj_id, fid, pat_id, rid) =
        seed_promote_fixture(&s, &suffix, "promote_pattern", true).await;

    s.accept_recommendation(&rid).await.unwrap();
    let err = s.accept_recommendation(&rid).await.expect_err("second accept is guarded at pending");
    assert!(
        err.contains("already decided") || err.contains("not found"),
        "guard error text: {err}"
    );

    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(p["lifecycle"], "rule", "promotion fired once; the guarded re-accept is inert");

    cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
}

/// (5) Read-path: after accept, get_workspace_patterns surfaces the pattern
/// with kind='adopted' (pattern_kind maps lifecycle='rule' → adopted).
#[tokio::test]
async fn accept_promote_pattern_reads_back_as_adopted() {
    let s = pg_store().await;
    let suffix = format!("accept_adopted_{}", uuid::Uuid::new_v4());
    let (proj_id, _fid, pat_id, rid) =
        seed_promote_fixture(&s, &suffix, "promote_pattern", true).await;

    s.accept_recommendation(&rid).await.unwrap();

    let view = s.get_workspace_patterns(&proj_id).await.unwrap();
    let followed = view["followed"].as_array().expect("followed array");
    let p = followed
        .iter()
        .find(|p| p["id"] == pat_id.to_string())
        .expect("promoted pattern in followed set");
    assert_eq!(p["kind"], "adopted", "lifecycle='rule' reads back as adopted");
    assert_eq!(p["lifecycle"], "rule");

    cleanup_promote_fixture(&s, &proj_id, &pat_id, &rid).await;
}

// ── Verdict regression → challenge the source memory ────────────────
// When an accepted rec's FTR REGRESSES after acceptance, the memory that
// spawned it (via based_on.patterns[0] → memories.source_id) is challenged
// (weakened) through the existing memory_outcome pipeline.

/// Seed a (workspace, folder, pattern, learned memory sourced by the pattern)
/// fixture. The memory starts at 1.0 + `strength_bump` so a single violation
/// (−0.7) challenges rather than archives it. Returns (proj, folder, pat, mem).
async fn seed_pattern_and_sourced_memory(
    s: &PgStore,
    suffix: &str,
    strength_bump: f64,
) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let (proj_id, fid) = create_test_workspace_and_folder(s, suffix).await;
    let pat_id = s
        .upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid),
            name: "_test:regressed-rule",
            is_anti: false,
            confidence: None,
            instances: &serde_json::json!([]),
        })
        .await
        .unwrap();
    // Convention memory sourced by the pattern — mirrors the rule-candidates generator.
    let mem = InsertMemory {
        workspace_id: Some(proj_id),
        scope: "workspace".to_string(),
        scope_filter: None,
        mtype: "convention".to_string(),
        title: format!("_test:regressed-memory:{suffix}"),
        content: "always foo".to_string(),
        impact: None,
        tags: Vec::new(),
        triage_signal: None,
        status: "active".to_string(),
        namespace_id: None,
        enforcement: None,
        origin: Some("learned".to_string()),
        source_id: Some(pat_id),
        spine_slot: None,
        feature: None,
    };
    let mem_id = s.insert_memory(&mem).await.unwrap();
    if strength_bump > 0.0 {
        s.reinforce_memory(&mem_id, strength_bump).await.unwrap();
    }
    (proj_id, fid, pat_id, mem_id)
}

/// Extend the memory fixture with an accepted+regressed promote_pattern rec:
/// acted 4 days ago at baseline FTR 0.9, then ≥3 post-acceptance sessions that
/// all fail (ftr=false) so the measured current FTR is 0.0 → a negative verdict.
async fn seed_regressed_rec_with_memory(
    s: &PgStore,
    suffix: &str,
) -> (uuid::Uuid, uuid::Uuid, uuid::Uuid, uuid::Uuid) {
    let (proj_id, fid, pat_id, mem_id) = seed_pattern_and_sourced_memory(s, suffix, 2.0).await;
    let based_on = serde_json::json!({ "patterns": [pat_id] });
    let rec_id = s
        .create_recommendation_full(&RecommendationRow {
            workspace_id: &proj_id,
            title: "_test:regressed-rec",
            why: "why",
            impact: None,
            action_type: "promote_pattern",
            urgency: "medium",
            based_on: &based_on,
            reasoning_trace_id: None,
            prompt: None,
        })
        .await
        .unwrap();
    sqlx_core::query::query(
        "UPDATE inference.recommendations
                SET status = 'accepted'::sensei.recommendation_status,
                    acted_at = now() - interval '4 days',
                    baseline_ftr = 0.900
              WHERE id = $1",
    )
    .bind(rec_id)
    .execute(s.pool())
    .await
    .unwrap();
    for _ in 0..3 {
        sqlx_core::query::query(
            "INSERT INTO activity.sessions (folder_id, workspace_id, outcome, ftr, started_at)
                 VALUES ($1, $2, 'corrected'::sensei.session_outcome, false, now())",
        )
        .bind(fid)
        .bind(proj_id)
        .execute(s.pool())
        .await
        .unwrap();
    }
    (proj_id, pat_id, mem_id, rec_id)
}

async fn cleanup_regressed_fixture(s: &PgStore, proj_id: &uuid::Uuid) {
    // Sessions FK to the folder (persisted), not the workspace, so drop them
    // explicitly; delete_workspace cascades recs, memories(+outcomes), patterns.
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE workspace_id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
    s.delete_workspace(proj_id).await.ok();
}

async fn violated_count(s: &PgStore, mem_id: &uuid::Uuid) -> i64 {
    let row: (i64,) = query_as(
        "SELECT count(*) FROM sensei.memory_outcomes WHERE memory_id = $1 AND outcome = 'violated'",
    )
    .bind(mem_id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    row.0
}

/// Full round-trip: measuring a regressed rec flips its verdict to negative
/// AND challenges the source memory exactly once — re-measuring is inert
/// (the rec is no longer pending, so the transition never re-fires).
#[tokio::test]
async fn measure_regressed_rec_challenges_source_memory_once() {
    let s = pg_store().await;
    let suffix = format!("regress_challenge_{}", uuid::Uuid::new_v4());
    let (proj_id, _pat_id, mem_id, _rec_id) = seed_regressed_rec_with_memory(&s, &suffix).await;

    let m0 = s.get_memory(&mem_id).await.unwrap().unwrap();
    assert!((m0["strength"].as_f64().unwrap() - 3.0).abs() < 1e-6, "seed strength 3.0");
    assert_eq!(m0["status"], "active", "memory starts active");

    s.measure_pending_verdicts().await.unwrap();

    let recs = s.list_recommendations(&proj_id).await.unwrap();
    let r = recs.iter().find(|r| r["title"] == "_test:regressed-rec").unwrap();
    assert_eq!(r["verdict"], "negative", "FTR dropped 0.9→0.0 → negative verdict");

    assert_eq!(
        violated_count(&s, &mem_id).await,
        1,
        "one violation recorded for the source memory"
    );
    let m1 = s.get_memory(&mem_id).await.unwrap().unwrap();
    assert!(
        (m1["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6,
        "trigger dropped strength 3.0→2.3"
    );
    assert_eq!(m1["status"], "challenged", "trigger moved memory to challenged");

    // Re-measure: the rec is no longer pending → not re-measured → no second hit.
    s.measure_pending_verdicts().await.unwrap();
    assert_eq!(violated_count(&s, &mem_id).await, 1, "idempotent: no second violation on re-run");
    let m2 = s.get_memory(&mem_id).await.unwrap().unwrap();
    assert!((m2["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6, "strength did not drop again");

    cleanup_regressed_fixture(&s, &proj_id).await;
}

/// Method-level idempotency: the `rec:<id>` context marker gates the write, so
/// challenging the same memory for the same rec twice records only one violation.
#[tokio::test]
async fn challenge_source_memory_for_rec_is_idempotent_per_rec() {
    let s = pg_store().await;
    let suffix = format!("challenge_idem_{}", uuid::Uuid::new_v4());
    let (proj_id, _fid, pat_id, mem_id) = seed_pattern_and_sourced_memory(&s, &suffix, 2.0).await;
    let based_on = serde_json::json!({ "patterns": [pat_id] }).to_string();
    let rec_id = uuid::Uuid::new_v4();

    assert!(
        s.challenge_source_memory_for_rec(&rec_id, &based_on).await.unwrap(),
        "first challenge records a violation"
    );
    assert_eq!(violated_count(&s, &mem_id).await, 1);
    let m1 = s.get_memory(&mem_id).await.unwrap().unwrap();
    assert!((m1["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6, "strength 3.0→2.3");
    assert_eq!(m1["status"], "challenged");

    // Same rec again → no-op, no second violation, strength unchanged.
    assert!(
        !s.challenge_source_memory_for_rec(&rec_id, &based_on).await.unwrap(),
        "second challenge for same rec is a no-op"
    );
    assert_eq!(violated_count(&s, &mem_id).await, 1, "still exactly one violation");
    let m2 = s.get_memory(&mem_id).await.unwrap().unwrap();
    assert!((m2["strength"].as_f64().unwrap() - 2.3).abs() < 1e-6, "strength did not drop again");

    cleanup_regressed_fixture(&s, &proj_id).await;
}

#[tokio::test]
async fn reinforce_source_memory_for_rec_bumps_promotes_and_is_idempotent() {
    // The G1→G2 bridge: a positive verdict reinforces the source memory via an
    // `applied` outcome, and the memory_outcome_apply trigger promotes it up
    // the ladder. Seed strength 3.6 (active) → one applied → 4.1 (≥4.0, no
    // violations) → battle_tested. Second call for the same rec is a no-op.
    let s = pg_store().await;
    let suffix = format!("reinforce_{}", uuid::Uuid::new_v4());
    let (proj_id, _fid, pat_id, mem_id) = seed_pattern_and_sourced_memory(&s, &suffix, 2.6).await;
    let based_on = serde_json::json!({ "patterns": [pat_id] }).to_string();
    let rec_id = uuid::Uuid::new_v4();

    let (before,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT reinforced_count::bigint FROM sensei.memories WHERE id=$1",
    )
    .bind(mem_id)
    .fetch_one(s.pool())
    .await
    .unwrap();

    assert!(
        s.reinforce_source_memory_for_rec(&rec_id, &based_on).await.unwrap(),
        "first reinforce records an applied outcome"
    );
    let (strength, status, count): (f64, String, i64) = sqlx_core::query_as::query_as(
            "SELECT strength::float8, status::text, reinforced_count::bigint FROM sensei.memories WHERE id=$1"
        ).bind(mem_id).fetch_one(s.pool()).await.unwrap();
    assert!((strength - 4.1).abs() < 1e-6, "strength 3.6→4.1, got {strength}");
    assert_eq!(status, "battle_tested", "promoted once strength >= 4.0 with no violations");
    assert_eq!(count, before + 1, "reinforced_count bumped once");

    // Same rec again → no-op (idempotency marker), count unchanged.
    assert!(
        !s.reinforce_source_memory_for_rec(&rec_id, &based_on).await.unwrap(),
        "second reinforce for same rec is a no-op"
    );
    let (count2,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT reinforced_count::bigint FROM sensei.memories WHERE id=$1",
    )
    .bind(mem_id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(count2, before + 1, "reinforced_count unchanged on idempotent re-run");

    cleanup_regressed_fixture(&s, &proj_id).await;
}

/// A rec with no resolvable source memory is a clean no-op (not an error):
/// a pattern that never spawned a memory, and empty/absent provenance.
#[tokio::test]
async fn challenge_source_memory_for_rec_no_source_memory_is_noop() {
    let s = pg_store().await;
    let suffix = format!("challenge_nomem_{}", uuid::Uuid::new_v4());
    let (proj_id, fid) = create_test_workspace_and_folder(&s, &suffix).await;
    // Pattern with NO sourced memory.
    let pat_id = s
        .upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid),
            name: "_test:orphan-rule",
            is_anti: false,
            confidence: None,
            instances: &serde_json::json!([]),
        })
        .await
        .unwrap();
    let rec_id = uuid::Uuid::new_v4();

    let based_on = serde_json::json!({ "patterns": [pat_id] }).to_string();
    assert!(
        !s.challenge_source_memory_for_rec(&rec_id, &based_on).await.unwrap(),
        "no memory sources this pattern → no-op"
    );
    // Empty / absent provenance → no-op, no panic.
    assert!(!s.challenge_source_memory_for_rec(&rec_id, r#"{"patterns":[]}"#).await.unwrap());
    assert!(!s.challenge_source_memory_for_rec(&rec_id, "{}").await.unwrap());
    assert!(
        s.memory_id_by_source(&pat_id).await.unwrap().is_none(),
        "sanity: pattern has no learned memory"
    );

    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(pat_id)
        .execute(s.pool())
        .await
        .ok();
    s.delete_workspace(&proj_id).await.ok();
}

// ── P-A.2: canonical sensei.repositories schema invariants ───────────
/// The repo-grain foundation: repositories are keyed on a normalized remote
/// (unique repo_key), a remote-less repo is NULL (multiple coexist), and a
/// folder's repository_id is SET NULL when its repository is deleted (I10) so
/// the checkout survives while the metric grain is re-resolved. Locks the
/// schema P-A.3's compute + upsert depend on.
#[tokio::test]
async fn repositories_schema_invariants() {
    let s = PgStore::connect_test().await.unwrap();
    let tag = uuid::Uuid::new_v4();
    let key = format!("host/{tag}/repo");

    // NULL repo_key = local-only, no remote: MANY coexist (nulls distinct).
    let (n1,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES (NULL, 'local-a') RETURNING id",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    let (n2,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES (NULL, 'local-b') RETURNING id",
    )
    .fetch_one(s.pool())
    .await
    .expect("two NULL repo_keys must coexist");

    // A non-null repo_key is UNIQUE: the second insert of the same key fails.
    let (r1,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'canon') RETURNING id",
    )
    .bind(&key)
    .fetch_one(s.pool())
    .await
    .unwrap();
    let dup = sqlx_core::query::query(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'dup')",
    )
    .bind(&key)
    .execute(s.pool())
    .await;
    assert!(dup.is_err(), "duplicate repo_key must violate the unique constraint");

    // folders.repository_id → ON DELETE SET NULL: deleting the repository
    // leaves the checkout folder but clears its (now-stale) repository_id.
    let root_id = s
        .add_watch_root(&format!("/tmp/repo-inv-{tag}"), "inv", &serde_json::json!([]))
        .await
        .unwrap();
    let fid = s
        .upsert_repo_kind(&root_id, "git", "co", &format!("/tmp/repo-inv-{tag}/co"))
        .await
        .unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET repository_id = $1 WHERE id = $2")
        .bind(r1)
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
        .bind(r1)
        .execute(s.pool())
        .await
        .unwrap();
    let (after,): (Option<uuid::Uuid>,) =
        query_as("SELECT repository_id FROM sensei.folders WHERE id = $1")
            .bind(fid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(
        after.is_none(),
        "deleting a repository must SET NULL folders.repository_id, not delete the folder"
    );

    // cleanup (shared test DB)
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = ANY($1)")
        .bind(vec![n1, n2])
        .execute(s.pool())
        .await
        .ok();
}

// ── P-A.3b Stage 0: repository population (folders.repository_id) ─────
/// A git checkout with a remote is linked to a canonical `sensei.repositories`
/// row keyed on the NORMALIZED remote; a remote-less checkout stays NULL
/// (local-only, never federated); re-running is a no-op (idempotent).
/// A folder registered INSIDE another, where BOTH hold nodes for the same files,
/// is reported — and a nested folder whose nodes were already healed away is NOT.
///
/// Keyed on NODES, not `files`, and that is the whole accuracy of the check.
/// A first version compared `files.content_hash` and reported seven cases on
/// the live index; SIX were false. `heal_nested_standalone_roots` deletes a
/// mis-scoped root's nodes and re-classifies the folder but leaves its
/// `files` rows, so a folder healed long ago still looks fully duplicated by
/// content while holding one module-container node — `cluster/server` measured 1
/// node against 1,970 stale `files` rows. Only `acme-corp/documentation`
/// (its own git checkout, 4,311 nodes inside the `acme-corp` repo) was real.
///
/// Breaking mutation: key the query on `files` again, or drop the
/// `file_path` correspondence between outer and inner — the healed folder is
/// reported and the check goes back to being six-sevenths noise.
#[tokio::test]
async fn a_nested_folder_whose_files_are_indexed_twice_is_reported_but_a_healed_one_is_not() {
    let s = PgStore::connect_test().await.unwrap();
    let tag = uuid::Uuid::new_v4();
    let base = format!("/tmp/contain-{tag}");
    let root_id = s.add_watch_root(&base, "ct", &serde_json::json!([])).await.unwrap();

    let outer =
        s.upsert_repo_kind(&root_id, "git", "outer", &format!("{base}/outer")).await.unwrap();
    // Nested, and BOTH folders hold nodes for its files → genuinely indexed twice.
    let dup =
        s.upsert_repo_kind(&root_id, "git", "dup", &format!("{base}/outer/dup")).await.unwrap();
    // Nested, but HEALED: its content nodes were deleted, leaving only a module
    // container. Stale `files` rows alone must not resurrect it as a duplicate.
    let healed = s
        .upsert_repo_kind(&root_id, "git", "healed", &format!("{base}/outer/healed"))
        .await
        .unwrap();

    for i in 0..6 {
        // The outer repo indexes both subtrees, repo-relative.
        crate::tasks::test_support::seed_node(
            &s,
            &outer,
            "function",
            &format!("d{i}"),
            &format!("dup/f{i}.rs"),
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
        crate::tasks::test_support::seed_node(
            &s,
            &outer,
            "function",
            &format!("h{i}"),
            &format!("healed/f{i}.rs"),
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
        // The duplicate ALSO holds them, folder-relative.
        crate::tasks::test_support::seed_node(
            &s,
            &dup,
            "function",
            &format!("d{i}"),
            &format!("f{i}.rs"),
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
        // The healed one holds stale `files` rows but NO content nodes.
        s.upsert_scan_state(&crate::db::pg_store::FileFingerprint {
            folder_id: &healed,
            file_path: &format!("f{i}.rs"),
            mtime: 1,
            content_hash: &format!("hash-{tag}-{i}"),
        })
        .await
        .unwrap();
    }
    // ...only the module container the heal leaves behind.
    s.upsert_dir_node(&healed, "module", "healed", "").await.unwrap();

    let dups = s.contained_duplicate_folders().await.unwrap();
    let mine: Vec<_> = dups.iter().filter(|(o, _, _, _)| o.contains(&tag.to_string())).collect();

    assert_eq!(mine.len(), 1, "exactly one genuine duplicate expected, got {mine:?}");
    let (outer_path, inner_path, both, inner_files) = mine[0];
    assert!(outer_path.ends_with("/outer"), "{outer_path}");
    assert!(inner_path.ends_with("/outer/dup"), "{inner_path}");
    assert_eq!(
        (*both, *inner_files),
        (6, 6),
        "all six of the inner's files are in the graph twice"
    );

    assert!(
        !dups.iter().any(|(_, i, _, _)| i.ends_with("/outer/healed")),
        "a healed folder carries stale `files` rows but no duplicate nodes: {dups:?}"
    );
}

/// A symlinked checkout is grouped onto the real directory's repository, and the
/// result is then REPORTABLE as one repository at two paths.
///
/// `assign_repositories` keys on a git remote, and a symlinked twin has no
/// `.git` of its own — it is classified `standalone` and was skipped, left
/// `repository_id = NULL`, so no query could relate the two. Measured:
/// `~/Developer/sensei-hq/gateway` symlinks to `~/Developer/gateway`, one inode
/// tree indexed under two paths, sharing 4,543 fqns — the largest duplicate pair
/// in the graph and the entire rust duplicate population.
///
/// Covers the three pieces the handler composes: `folder_identities_for_root`
/// must return the standalone twin (not just git/subtree kinds),
/// `link_folders_to_repositories` must point it at the real folder's repository
/// and be idempotent, and `duplicate_repository_paths` must then report both
/// paths so the doctor can recommend removing one.
#[tokio::test]
async fn a_symlinked_checkout_is_grouped_and_then_reported_as_one_repository() {
    let s = PgStore::connect_test().await.unwrap();
    let tag = uuid::Uuid::new_v4();
    let root_id = s
        .add_watch_root(&format!("/tmp/symrepo-{tag}"), "sr", &serde_json::json!([]))
        .await
        .unwrap();

    // The real checkout, with a remote → gets a repository.
    let real = s
        .upsert_repo_kind(&root_id, "git", "gateway", &format!("/tmp/symrepo-{tag}/gateway"))
        .await
        .unwrap();
    let url = format!("git@github.com:Org/Gateway-{tag}.git");
    s.update_folder_remotes(&real, &serde_json::json!([{"name": "origin", "url": url}]))
        .await
        .unwrap();
    // The symlinked twin: STANDALONE, no remote — exactly what was skipped.
    let link = s
        .upsert_repo_kind(
            &root_id,
            "standalone",
            "gateway",
            &format!("/tmp/symrepo-{tag}/org/gateway"),
        )
        .await
        .unwrap();
    s.assign_repositories(&root_id).await.unwrap();

    // The twin must be VISIBLE to the identity read despite being standalone.
    let ids = s.folder_identities_for_root(&root_id).await.unwrap();
    assert!(
        ids.iter().any(|(id, _, repo)| *id == link && repo.is_none()),
        "the standalone symlink must be offered for grouping: {ids:?}"
    );
    let repo_id = ids
        .iter()
        .find(|(id, _, _)| *id == real)
        .and_then(|(_, _, r)| *r)
        .expect("the real checkout is linked to a repository");

    // Both paths canonicalise to the same real directory, so they group.
    let identities: Vec<crate::tasks::handlers::scan_logic::FolderPathIdentity> = ids
        .into_iter()
        .map(|(id, abs_path, repository_id)| {
            crate::tasks::handlers::scan_logic::FolderPathIdentity {
                id,
                real_path: format!("/tmp/symrepo-{tag}/gateway"),
                abs_path,
                repository_id,
            }
        })
        .collect();
    let links = crate::tasks::handlers::scan_logic::symlink_repository_links(&identities);
    assert_eq!(links, vec![(link, repo_id)], "the symlink adopts the real dir's repository");

    assert_eq!(s.link_folders_to_repositories(&links).await.unwrap(), 1, "one row changed");
    assert_eq!(
        s.link_folders_to_repositories(&links).await.unwrap(),
        0,
        "idempotent — a settled registry writes nothing"
    );

    // And now it is reportable: one repository, two paths.
    let dups = s.duplicate_repository_paths().await.unwrap();
    let mine = dups
        .iter()
        .find(|(_, url, _)| url.as_deref().is_some_and(|u| u.contains(&tag.to_string())))
        .expect("the grouped repository is reported as duplicated");
    assert_eq!(mine.2.len(), 2, "both paths are named so the user can pick one: {:?}", mine.2);
    assert!(mine.2.iter().any(|p| p.ends_with("/org/gateway")), "{:?}", mine.2);
}

#[tokio::test]
async fn assign_repositories_links_folders_to_canonical_repo_key() {
    let s = PgStore::connect_test().await.unwrap();
    let tag = uuid::Uuid::new_v4();
    let root_id = s
        .add_watch_root(&format!("/tmp/assignrepo-{tag}"), "ar", &serde_json::json!([]))
        .await
        .unwrap();

    // A git checkout WITH a remote (run-unique so it can't collide in the shared DB).
    let with_remote = s
        .upsert_repo_kind(&root_id, "git", "repo", &format!("/tmp/assignrepo-{tag}/repo"))
        .await
        .unwrap();
    let url = format!("git@github.com:Org/Repo-{tag}.git");
    s.update_folder_remotes(&with_remote, &serde_json::json!([{"name": "origin", "url": url}]))
        .await
        .unwrap();
    // A git checkout with NO remote → local-only.
    let no_remote = s
        .upsert_repo_kind(&root_id, "git", "local", &format!("/tmp/assignrepo-{tag}/local"))
        .await
        .unwrap();

    let n = s.assign_repositories(&root_id).await.unwrap();
    assert_eq!(
        n, 1,
        "only the folder with a remote is (re)pointed; the remote-less one stays NULL"
    );

    // The remote folder → a repositories row keyed on the NORMALIZED remote.
    let (rid, key): (Option<uuid::Uuid>, Option<String>) = query_as(
        "SELECT f.repository_id, r.repo_key FROM sensei.folders f \
               LEFT JOIN sensei.repositories r ON r.id = f.repository_id WHERE f.id = $1",
    )
    .bind(with_remote)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(rid.is_some(), "the git checkout with a remote is linked to a repository");
    assert_eq!(
        key.as_deref(),
        Some(format!("github.com/org/repo-{tag}").as_str()),
        "linked via the normalized remote key (scheme/creds/.git stripped, lowercased)"
    );

    // The remote-less folder stays NULL — local-only, never federated.
    let (rid2,): (Option<uuid::Uuid>,) =
        query_as("SELECT repository_id FROM sensei.folders WHERE id = $1")
            .bind(no_remote)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(rid2.is_none(), "a checkout with no remote is left repository_id NULL");

    // Idempotent: a second pass re-points nothing.
    assert_eq!(
        s.assign_repositories(&root_id).await.unwrap(),
        0,
        "re-run is a no-op (no folder changed)"
    );

    // cleanup (delete the watch root → cascades folders; then the repository row)
    let repo_id = rid.unwrap();
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
        .bind(repo_id)
        .execute(s.pool())
        .await
        .ok();
}

// ── P-A.3b Stage 1: metric_watermarks ────────────────────────────────
/// The watermark is keyed uniquely per (repository, metric_group) and cascades
/// when its repository is deleted (so a repo prune leaves no orphan cursor).
#[tokio::test]
async fn metric_watermarks_pk_and_cascade_on_repository_delete() {
    let s = PgStore::connect_test().await.unwrap();
    let tag = uuid::Uuid::new_v4();
    let (rid,): (uuid::Uuid,) =
        query_as("INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'wm') RETURNING id")
            .bind(format!("host/wm/{tag}"))
            .fetch_one(s.pool())
            .await
            .unwrap();

    sqlx_core::query::query(
        "INSERT INTO sensei.metric_watermarks (repository_id, metric_group, sealed_through) \
             VALUES ($1, 'session_outcomes', current_date)",
    )
    .bind(rid)
    .execute(s.pool())
    .await
    .unwrap();

    // PK: a duplicate (repository_id, metric_group) is rejected.
    let dup = sqlx_core::query::query(
            "INSERT INTO sensei.metric_watermarks (repository_id, metric_group) VALUES ($1, 'session_outcomes')",
        ).bind(rid).execute(s.pool()).await;
    assert!(dup.is_err(), "duplicate (repository_id, metric_group) violates the PK");

    // FK cascade: deleting the repository drops its watermark (no orphan cursor).
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();
    let (cnt,): (i64,) =
        query_as("SELECT count(*) FROM sensei.metric_watermarks WHERE repository_id = $1")
            .bind(rid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(cnt, 0, "watermarks cascade on repository delete");
}

// ── P-A.3b Stage 2 foundation: workspace→repositories resolution ────────
/// A workspace spans the distinct repositories on its checkout folders, listed
/// shallowest-first (the "primary" = root checkout's repo, where workspace-level
/// metrics attach); the multi-repo compute iterates this, not one root path.
#[tokio::test]
async fn repositories_for_workspace_lists_repos_primary_first() {
    let s = PgStore::connect_test().await.unwrap();
    let tag = uuid::Uuid::new_v4();
    let root_id =
        s.add_watch_root(&format!("/tmp/rfp-{tag}"), "rfp", &serde_json::json!([])).await.unwrap();
    let pid = s.create_workspace(&format!("_test:rfp:{tag}"), None, None).await.unwrap();

    let (r_root,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'root') RETURNING id",
    )
    .bind(format!("host/{tag}/root"))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_workspace(&s, &r_root, &pid, "root").await;
    let (r_nested,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'nested') RETURNING id",
    )
    .bind(format!("host/{tag}/nested"))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_workspace(&s, &r_nested, &pid, "nested").await;
    // Root checkout (shallow path) + a nested checkout (deeper path), both in the workspace.
    let f_root =
        s.upsert_repo_kind(&root_id, "git", "root", &format!("/tmp/rfp-{tag}/root")).await.unwrap();
    let f_nested = s
        .upsert_repo_kind(&root_id, "git", "nested", &format!("/tmp/rfp-{tag}/root/vendor/nested"))
        .await
        .unwrap();
    for (f, r) in [(f_root, r_root), (f_nested, r_nested)] {
        // Only the repository link. The MEMBERSHIP was already written, against
        // the repository, by the two `link_repository_to_workspace` calls above —
        // that is the only place it exists now.
        sqlx_core::query::query("UPDATE sensei.folders SET repository_id = $2 WHERE id = $1")
            .bind(f)
            .bind(r)
            .execute(s.pool())
            .await
            .unwrap();
    }

    let repos = s.repositories_for_workspace(&pid).await.unwrap();
    assert_eq!(repos, vec![r_root, r_nested], "both repos, shallowest (primary) first");
    assert_eq!(
        s.primary_repository_for_workspace(&pid).await.unwrap(),
        Some(r_root),
        "primary repository = the shallowest checkout's repository"
    );

    // cleanup
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = ANY($1)")
        .bind(vec![r_root, r_nested])
        .execute(s.pool())
        .await
        .ok();
}

/// #287: the Patterns view folds the unreviewed `rework: <file>` rows into one
/// "rework hotspots" anti-pattern; a rework row the user promoted, and every
/// other pattern, still reads as itself. The per-file rows stay in the table.
#[tokio::test]
async fn workspace_patterns_fold_unreviewed_rework_into_hotspots() {
    let s = pg_store().await;
    let suffix = format!("rework_fold_{}", uuid::Uuid::new_v4());
    let (proj_id, fid) = create_test_workspace_and_folder(&s, &suffix).await;
    let none = serde_json::json!([]);
    for name in ["rework: crates/a.rs", "rework: docs/b.md"] {
        s.upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid),
            name,
            is_anti: true,
            confidence: Some(0.6),
            instances: &none,
        })
        .await
        .unwrap();
    }
    let kept = s
        .upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid),
            name: "rework: crates/kept.rs",
            is_anti: true,
            confidence: Some(0.9),
            instances: &none,
        })
        .await
        .unwrap();
    s.promote_pattern(&kept, "rule").await.unwrap();
    s.upsert_pattern(&crate::db::pg_store::PatternRow {
        workspace_id: &proj_id,
        folder_id: Some(&fid),
        name: "correction-prone",
        is_anti: true,
        confidence: Some(0.5),
        instances: &none,
    })
    .await
    .unwrap();

    let view = s.get_workspace_patterns(&proj_id).await.unwrap();
    let mut anti: Vec<(String, i64)> = view["antiPatterns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (p["name"].as_str().unwrap().to_string(), p["instanceCount"].as_i64().unwrap_or(-1))
        })
        .collect();
    anti.sort();

    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE workspace_id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();

    let names: Vec<&str> = anti.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["correction-prone", "rework hotspots", "rework: crates/kept.rs"]);
    assert_eq!(anti.iter().find(|(n, _)| n == "rework hotspots").unwrap().1, 2);
}
