use super::*;

// These connect via `PgStore::connect_test()` like the rest of the suite.
//
// They used to open with `if ddl_test_skip() { return; }`, gated on a
// `SENSEI_TEST_DB_URL` env var that is set nowhere in the repo or in any
// shell profile — so all ten returned before asserting anything and reported
// green forever. A test that cannot fail is worse than no test: it claims
// coverage of memory retrieval, context assembly and 7d telemetry that was
// never actually being exercised.

#[tokio::test]
async fn list_memories_filters_by_status() {
    let pg = PgStore::connect_test().await.unwrap();
    let project_id = pg.ensure_test_project("list-status").await.unwrap();
    let m1 = pg
        .insert_memory(&InsertMemory {
            project_id: Some(project_id),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "t1".into(),
            content: "c1".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
            status: "proposed".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let _m2 = pg
        .insert_memory(&InsertMemory {
            project_id: Some(project_id),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "t2".into(),
            content: "c2".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();

    let proposed = pg.list_memories(Some(project_id), Some("proposed"), None, 50).await.unwrap();
    assert_eq!(proposed.len(), 1);
    assert_eq!(proposed[0]["id"].as_str().unwrap(), m1.to_string());

    // `list-status` is a reused fixture project (ensure_test_project, #34) —
    // clean up so repeated runs don't accrete proposed rows into the count.
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
        .bind(&[m1, _m2][..])
        .execute(pg.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn set_memory_status_accept_proposal() {
    let pg = PgStore::connect_test().await.unwrap();
    let pid = pg.ensure_test_project("accept-prop").await.unwrap();
    let mid = pg
        .insert_memory(&InsertMemory {
            project_id: Some(pid),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "t".into(),
            content: "c".into(),
            impact: None,
            tags: vec![],
            triage_signal: Some("revert".into()),
            status: "proposed".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();

    let new_status = pg.set_memory_status(mid, "active", &["proposed"]).await.unwrap();
    assert_eq!(new_status.as_deref(), Some("active"));

    // Trying to accept a now-active memory fails.
    let err = pg.set_memory_status(mid, "active", &["proposed"]).await;
    assert!(err.is_err() || err.unwrap().is_none(), "second accept should not match WHERE clause");
}

#[tokio::test]
async fn get_memory_detail_includes_outcomes() {
    let pg = PgStore::connect_test().await.unwrap();
    let pid = pg.ensure_test_project("detail").await.unwrap();
    let mid = pg
        .insert_memory(&InsertMemory {
            project_id: Some(pid),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "t".into(),
            content: "c".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let skipped = pg
        .record_outcomes_batch(&[OutcomeRow {
            memory_id: mid,
            session_id: None,
            outcome: "applied".into(),
            context: None,
        }])
        .await
        .unwrap();
    assert_eq!(skipped.len(), 0);

    let detail = pg.get_memory_detail(mid).await.unwrap();
    assert!(detail["memory"]["id"].as_str().unwrap() == mid.to_string());
    assert_eq!(detail["outcomes"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn assemble_context_blends_three_scopes() {
    let pg = PgStore::connect_test().await.unwrap();
    let pid = pg.ensure_test_project("blend").await.unwrap();

    // Scope every query in this test to a run-unique tag.
    //
    // `assemble_context` matches `project_id = $2 OR scope='stack' OR
    // scope='global'` and then takes the top N by strength, so the shared test
    // DB's several-hundred global memories crowd this test's fixtures out of
    // the window — deleting our own rows afterwards (below) never fixed that,
    // because the noise is other suites' rows, not ours. The tags filter is
    // part of the real contract, so the three-scope blend is still exercised;
    // each fixture still qualifies via a DIFFERENT scope branch.
    let tag = format!("blend-fixture-{}", uuid::Uuid::new_v4());
    let only_ours = vec![tag.clone()];

    let m_p = pg
        .insert_memory(&InsertMemory {
            project_id: Some(pid),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "P".into(),
            content: "p".into(),
            impact: None,
            tags: vec![tag.clone()],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let m_s = pg
        .insert_memory(&InsertMemory {
            project_id: None,
            scope: "stack".into(),
            scope_filter: Some("rust".into()),
            mtype: "convention".into(),
            title: "S".into(),
            content: "s".into(),
            impact: None,
            tags: vec![tag.clone()],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let m_g = pg
        .insert_memory(&InsertMemory {
            project_id: None,
            scope: "global".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "G".into(),
            content: "g".into(),
            impact: None,
            tags: vec![tag.clone()],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();

    let blob =
        pg.assemble_context(pid, &["rust".into()], Some(&only_ours), 50, None).await.unwrap();
    let titles: Vec<String> = blob["memories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["title"].as_str().unwrap().to_string())
        .collect();
    assert!(titles.contains(&"P".to_string()));
    assert!(titles.contains(&"S".to_string()));
    assert!(titles.contains(&"G".to_string()));

    // Proposed memories must not appear.
    let m_prop = pg
        .insert_memory(&InsertMemory {
            project_id: Some(pid),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "PROP".into(),
            content: "x".into(),
            impact: None,
            tags: vec![tag.clone()],
            triage_signal: Some("revert".into()),
            status: "proposed".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    // Carries the tag, so its absence below is genuinely the status filter
    // doing the work rather than the tag filter hiding it.
    let blob2 =
        pg.assemble_context(pid, &["rust".into()], Some(&only_ours), 50, None).await.unwrap();
    let titles2: Vec<String> = blob2["memories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["title"].as_str().unwrap().to_string())
        .collect();
    assert!(!titles2.contains(&"PROP".to_string()));

    // `blend` is a reused fixture project (#34) and "S"/"G" are global/stack
    // scoped — visible to every project. Clean up so repeated runs don't
    // accrete rows that eventually push this test's own memories out of
    // assemble_context's top-N window.
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
        .bind(&[m_p, m_s, m_g, m_prop][..])
        .execute(pg.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn list_memories_for_slot_matches_slot_and_feature() {
    let pg = PgStore::connect_test().await.unwrap();
    let pid = pg.ensure_test_project("slot-retrieval").await.unwrap();

    let m_design = pg
        .create_memory(
            Some(&pid),
            "project",
            None,
            "decision",
            "design-project-scope",
            "c",
            None,
            None,
            Some("design"),
            None,
        )
        .await
        .unwrap();
    let m_design_auth = pg
        .create_memory(
            Some(&pid),
            "project",
            None,
            "decision",
            "design-auth-feature",
            "c",
            None,
            None,
            Some("design"),
            Some("auth"),
        )
        .await
        .unwrap();
    let m_decisions = pg
        .create_memory(
            Some(&pid),
            "project",
            None,
            "decision",
            "decisions-project-scope",
            "c",
            None,
            None,
            Some("decisions"),
            None,
        )
        .await
        .unwrap();

    let design_project = pg.list_memories_for_slot(&pid, "design", None, 50).await.unwrap();
    assert_eq!(design_project.len(), 1);
    assert_eq!(design_project[0]["id"].as_str().unwrap(), m_design.to_string());

    let design_auth = pg.list_memories_for_slot(&pid, "design", Some("auth"), 50).await.unwrap();
    assert_eq!(design_auth.len(), 1);
    assert_eq!(design_auth[0]["id"].as_str().unwrap(), m_design_auth.to_string());

    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
        .bind(&[m_design, m_design_auth, m_decisions][..])
        .execute(pg.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn assemble_context_leads_with_slot_anchored_memory() {
    let pg = PgStore::connect_test().await.unwrap();
    let pid = pg.ensure_test_project("slot-leads").await.unwrap();

    // Tag-scoped for the same reason as `assemble_context_blends_three_scopes`:
    // the shared test DB's global memories otherwise fill the top-N window and
    // neither fixture appears. Uses `insert_memory` rather than `create_memory`
    // because only the former carries tags alongside `spine_slot`.
    let tag = format!("slot-leads-fixture-{}", uuid::Uuid::new_v4());
    let only_ours = vec![tag.clone()];
    let fixture = |title: &str, slot: Option<&str>| InsertMemory {
        project_id: Some(pid),
        scope: "project".into(),
        scope_filter: None,
        mtype: "decision".into(),
        title: title.into(),
        content: "c".into(),
        impact: None,
        tags: vec![tag.clone()],
        triage_signal: None,
        status: "active".into(),
        namespace_id: None,
        enforcement: None,
        origin: None,
        source_id: None,
        spine_slot: slot.map(|s| s.to_string()),
        feature: None,
    };

    // Unanchored memory created first so a strength/recency-only ordering
    // would put it ahead of the slot-anchored one below.
    let m_unanchored = pg.insert_memory(&fixture("unanchored", None)).await.unwrap();
    let m_design = pg.insert_memory(&fixture("design-anchored", Some("design"))).await.unwrap();

    let blob =
        pg.assemble_context(pid, &[], Some(&only_ours), 50, Some(("design", None))).await.unwrap();
    let ids: Vec<String> = blob["memories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        ids.first().map(String::as_str),
        Some(m_design.to_string().as_str()),
        "slot-anchored memory must lead the assembled bundle"
    );
    assert!(ids.contains(&m_unanchored.to_string()), "general blend still present");

    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = ANY($1)")
        .bind(&[m_unanchored, m_design][..])
        .execute(pg.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn assemble_context_logs_one_load_per_memory() {
    let pg = PgStore::connect_test().await.unwrap();
    let pid = pg.ensure_test_project("loads-writer").await.unwrap();
    // Project-scoped active memory → loaded exactly once per assemble_context
    // call on this (test-unique) project.
    // Tag-scoped so the delivered set is exactly this memory: the telemetry
    // assertions below count load rows, and an unscoped call would also log a
    // load for every unrelated global memory that fit in the window.
    let tag = format!("loads-fixture-{}", uuid::Uuid::new_v4());
    let only_ours = vec![tag.clone()];
    let mid = pg
        .insert_memory(&InsertMemory {
            project_id: Some(pid),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "L".into(),
            content: "l".into(),
            impact: None,
            tags: vec![tag.clone()],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();

    let blob = pg.assemble_context(pid, &[], Some(&only_ours), 50, None).await.unwrap();
    // Context is still delivered (writer is additive, non-fatal).
    assert!(
        blob["memories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"].as_str() == Some(&mid.to_string()))
    );

    let (loaded, followed, skipped) = pg.memory_telemetry_7d(mid).await.unwrap();
    assert_eq!(loaded, 1, "one load row per delivered memory");
    assert_eq!(followed, 0);
    assert_eq!(skipped, 0);

    // A second delivery logs a second load row. Same tag scope as the first —
    // an unscoped call may not deliver this memory at all, so it would log
    // nothing and the count below would stay at 1.
    pg.assemble_context(pid, &[], Some(&only_ours), 50, None).await.unwrap();
    let (loaded2, _, _) = pg.memory_telemetry_7d(mid).await.unwrap();
    assert_eq!(loaded2, 2);

    // Source + a non-null loaded_at are recorded; session_id NULL is tolerated.
    let (source, sess_null): (String, bool) = sqlx_core::query_as::query_as(
        "SELECT source, session_id IS NULL FROM activity.memory_loads
              WHERE memory_id = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(mid)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    assert_eq!(source, "get_layered_context");
    assert!(sess_null, "v1 logs loads with session_id NULL");
}

#[tokio::test]
async fn memory_loaded_last_7d_respects_window() {
    let pg = PgStore::connect_test().await.unwrap();
    let pid = pg.ensure_test_project("loads-window").await.unwrap();
    let mid = pg
        .insert_memory(&InsertMemory {
            project_id: Some(pid),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "W".into(),
            content: "w".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();

    // One load in-window, one back-dated outside the 7d window.
    sqlx_core::query::query("INSERT INTO activity.memory_loads (memory_id) VALUES ($1)")
        .bind(mid)
        .execute(pg.pool())
        .await
        .unwrap();
    sqlx_core::query::query(
            "INSERT INTO activity.memory_loads (memory_id, loaded_at) VALUES ($1, now() - interval '10 days')"
        ).bind(mid).execute(pg.pool()).await.unwrap();

    let (loaded, _, _) = pg.memory_telemetry_7d(mid).await.unwrap();
    assert_eq!(loaded, 1, "only the in-window load is counted");
}

#[tokio::test]
async fn memory_followed_skipped_last_7d_over_outcomes() {
    let pg = PgStore::connect_test().await.unwrap();
    let pid = pg.ensure_test_project("followed-skipped").await.unwrap();
    let mid = pg
        .insert_memory(&InsertMemory {
            project_id: Some(pid),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "F".into(),
            content: "f".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();

    // In-window outcomes: applied + ignored count; consulted + violated do not.
    for oc in ["applied", "ignored", "consulted", "violated"] {
        sqlx_core::query::query(
                "INSERT INTO sensei.memory_outcomes (memory_id, outcome) VALUES ($1, $2::sensei.memory_outcome)"
            ).bind(mid).bind(oc).execute(pg.pool()).await.unwrap();
    }
    // Back-dated applied must NOT count toward followed.
    sqlx_core::query::query(
        "INSERT INTO sensei.memory_outcomes (memory_id, outcome, recorded_at)
             VALUES ($1, 'applied'::sensei.memory_outcome, now() - interval '10 days')",
    )
    .bind(mid)
    .execute(pg.pool())
    .await
    .unwrap();

    let (loaded, followed, skipped) = pg.memory_telemetry_7d(mid).await.unwrap();
    assert_eq!(loaded, 0, "no loads logged in this test");
    assert_eq!(followed, 1, "only the in-window applied outcome");
    assert_eq!(skipped, 1, "only the in-window ignored outcome");
}

#[tokio::test]
async fn get_memory_detail_includes_7d_telemetry() {
    let pg = PgStore::connect_test().await.unwrap();
    let pid = pg.ensure_test_project("detail-telemetry").await.unwrap();
    let mid = pg
        .insert_memory(&InsertMemory {
            project_id: Some(pid),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "D".into(),
            content: "d".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: None,
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();

    sqlx_core::query::query("INSERT INTO activity.memory_loads (memory_id) VALUES ($1)")
        .bind(mid)
        .execute(pg.pool())
        .await
        .unwrap();
    for oc in ["applied", "ignored"] {
        sqlx_core::query::query(
                "INSERT INTO sensei.memory_outcomes (memory_id, outcome) VALUES ($1, $2::sensei.memory_outcome)"
            ).bind(mid).bind(oc).execute(pg.pool()).await.unwrap();
    }

    let detail = pg.get_memory_detail(mid).await.unwrap();
    assert_eq!(detail["loaded_last_7d"].as_i64().unwrap(), 1);
    assert_eq!(detail["followed_last_7d"].as_i64().unwrap(), 1);
    assert_eq!(detail["skipped_last_7d"].as_i64().unwrap(), 1);
}

#[tokio::test]
async fn insert_memory_persists_source_id() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let src = uuid::Uuid::new_v4();
    let id = pg
        .insert_memory(&InsertMemory {
            project_id: None,
            scope: "global".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "fed".into(),
            content: "federated content".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: Some("recommended".into()),
            origin: Some("federated".into()),
            source_id: Some(src),
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    let got: (Option<uuid::Uuid>,) =
        sqlx_core::query_as::query_as("SELECT source_id FROM sensei.memories WHERE id = $1")
            .bind(id)
            .fetch_one(pg.pool())
            .await
            .unwrap();
    assert_eq!(got.0, Some(src));
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(id)
        .execute(pg.pool())
        .await
        .unwrap();
}

/// Governance scope hygiene: a project's learned convention (an unscoped
/// memory carrying a `project_id`, exactly what the L2 generator writes in
/// `tasks::handlers::generate::generate_for_project`) must resolve ONLY for
/// its own project's repo — labeled `project`, not the always-on `general`
/// set — and must never bleed into another project's ruleset or the global
/// `~/.sensei/rules.md`. Regression for the cross-project general-rule bleed
/// found by dogfooding `get_rules`.
#[tokio::test]
async fn project_learned_convention_scopes_to_its_own_project_not_general() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let pool = pg.pool();

    // Two projects, each with its own repo folder attributed to it.
    let proj_a = pg
        .create_project(&format!("_test:rules-A-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let proj_b = pg
        .create_project(&format!("_test:rules-B-{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let root = pg
        .add_watch_root(
            &format!("/_test/rules-root-{}", uuid::Uuid::new_v4()),
            "t",
            &serde_json::json!([]),
        )
        .await
        .unwrap();
    let folder_a = pg
        .upsert_repo(&root, "rules-repo-a", &format!("/_test/rules-a-{}", uuid::Uuid::new_v4()))
        .await
        .unwrap();
    let folder_b = pg
        .upsert_repo(&root, "rules-repo-b", &format!("/_test/rules-b-{}", uuid::Uuid::new_v4()))
        .await
        .unwrap();
    pg.set_folder_project(&folder_a, &proj_a, "root", None).await.unwrap();
    pg.set_folder_project(&folder_b, &proj_b, "root", None).await.unwrap();

    // A learned convention captured for project A: namespace_id NULL,
    // project-tied — the shape the L2 generator emits.
    let conv_content = format!("project A convention {}", uuid::Uuid::new_v4());
    let conv = pg
        .insert_memory(&InsertMemory {
            project_id: Some(proj_a),
            scope: "project".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "conv A".into(),
            content: conv_content.clone(),
            impact: None,
            tags: vec![],
            triage_signal: Some("repeat_pattern".into()),
            status: "active".into(),
            namespace_id: None,
            enforcement: None,
            origin: Some("learned".into()),
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();

    // A genuinely-global rule: unscoped AND not tied to a project — the real
    // always-on set that must keep working.
    let global_content = format!("genuinely global rule {}", uuid::Uuid::new_v4());
    let global = pg
        .insert_memory(&InsertMemory {
            project_id: None,
            scope: "global".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "global rule".into(),
            content: global_content.clone(),
            impact: None,
            tags: vec![],
            triage_signal: None,
            status: "active".into(),
            namespace_id: None,
            enforcement: Some("recommended".into()),
            origin: Some("authored".into()),
            source_id: None,
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();

    // Project A's ruleset: A's convention (labeled `project`) + the global rule.
    let a_rules = pg.resolve_rules_raw(&folder_a).await.unwrap();
    let a_conv =
        a_rules.iter().find(|r| r.content == conv_content).expect("A's convention resolves for A");
    assert_eq!(
        a_conv.scope, "project",
        "a project-tied unscoped convention is labeled project, not general"
    );
    assert!(
        a_rules.iter().any(|r| r.content == global_content),
        "the genuinely-global rule applies to A"
    );

    // Project B's ruleset: MUST NOT contain A's convention; the global rule still applies.
    let b_rules = pg.resolve_rules_raw(&folder_b).await.unwrap();
    assert!(
        !b_rules.iter().any(|r| r.content == conv_content),
        "A's learned convention must NOT bleed into project B"
    );
    assert!(
        b_rules.iter().any(|r| r.content == global_content),
        "the genuinely-global rule still applies to B"
    );

    // Global always-on set: the genuinely-global rule, NOT any project convention.
    let global_set = pg.resolve_global_rules().await.unwrap();
    assert!(
        global_set.iter().any(|r| r.content == global_content),
        "genuinely-global rule is in the always-on set"
    );
    assert!(
        !global_set.iter().any(|r| r.content == conv_content),
        "a project convention must NOT be in the always-on global set"
    );

    // cleanup (best-effort)
    for id in [conv, global] {
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await
            .ok();
    }
    for f in [folder_a, folder_b] {
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
            .bind(f)
            .execute(pool)
            .await
            .ok();
    }
    sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
        .bind(root)
        .execute(pool)
        .await
        .ok();
    for p in [proj_a, proj_b] {
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(p)
            .execute(pool)
            .await
            .ok();
    }
}

#[tokio::test]
async fn federated_ledger_and_shareability() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    // Seed the scopes used by the test (sensei_test is empty; production data
    // is seeded via staging.import_scopes — we replicate the two rows we need).
    sqlx_core::query::query(
        "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('organization', 'Organization', 20, true),
                    ('technology',   'Technology',   40, false)
             ON CONFLICT (key) DO UPDATE SET shareable = EXCLUDED.shareable",
    )
    .execute(pg.pool())
    .await
    .unwrap();

    // organization is shareable; technology is not (seeded scopes ladder).
    let org_ns = pg.upsert_namespace("organization", "Test Org", "test-org-fed").await.unwrap();
    let tech_ns = pg.upsert_namespace("technology", "Rust", "rust-fed").await.unwrap();
    assert!(pg.namespace_is_shareable(&org_ns).await.unwrap());
    assert!(!pg.namespace_is_shareable(&tech_ns).await.unwrap());

    let src = pg
        .create_knowledge_source(&NewKnowledgeSource {
            kind: "hive_mind".into(),
            name: "H".into(),
            url: "u".into(),
            namespace_id: None,
            credential_ref: "c".into(),
            direction: "both".into(),
        })
        .await
        .unwrap();
    let remote = uuid::Uuid::new_v4();
    let mem = pg
        .insert_memory(&InsertMemory {
            project_id: None,
            scope: "global".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "t".into(),
            content: "c".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
            status: "active".into(),
            namespace_id: Some(org_ns),
            enforcement: Some("recommended".into()),
            origin: Some("federated".into()),
            source_id: Some(src),
            spine_slot: None,
            feature: None,
        })
        .await
        .unwrap();
    pg.upsert_federated_memory(&src, &remote, "hash1", Some(&mem), 5).await.unwrap();
    pg.upsert_federated_memory(&src, &remote, "hash1", Some(&mem), 9).await.unwrap(); // idempotent
    let link = pg.find_federated_memory(&src, &remote).await.unwrap().unwrap();
    assert_eq!(link.memory_id, Some(mem));
    assert_eq!(link.remote_seq, 9);

    // push payload: returns snapshot + namespace identity (incl. name) + origin/scope_key
    let payload = pg.memory_push_payload(&mem).await.unwrap().unwrap();
    assert_eq!(payload.scope_key, "organization");
    assert_eq!(payload.slug, "test-org-fed");
    assert_eq!(payload.name, "Test Org");
    assert_eq!(payload.origin, "federated");

    // archive retires a federated memory (drops out of resolution)
    assert!(pg.archive_federated_memory(&mem).await.unwrap());
    let (status,): (String,) =
        sqlx_core::query_as::query_as("SELECT status::text FROM sensei.memories WHERE id=$1")
            .bind(mem)
            .fetch_one(pg.pool())
            .await
            .unwrap();
    assert_eq!(status, "archived");

    pg.delete_knowledge_source(&src).await.unwrap(); // cascades the ledger row
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id=$1")
        .bind(mem)
        .execute(pg.pool())
        .await
        .unwrap();
    // clean up namespaces and seeded scopes
    sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = ANY($1::uuid[])")
        .bind(vec![org_ns, tech_ns])
        .execute(pg.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.scopes WHERE key IN ('organization','technology')")
        .execute(pg.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn latest_hook_event_ts_returns_max_for_family() {
    let pg = PgStore::connect_test().await.unwrap();
    let base = 1_900_000_000_000_i64; // far-future, won't collide with seeded data
    for (i, off) in [0_i64, 5000, 2000].iter().enumerate() {
        pg.insert_hook_event(
            &format!("sess-test-{i}"),
            "claude",
            "PreToolUse",
            Some("Bash"),
            Some("/tmp"),
            base + off,
            Some(true),
            &serde_json::json!({"t": i}),
        )
        .await
        .unwrap();
    }
    let max = pg.latest_hook_event_ts("claude").await.unwrap().unwrap();
    assert!(max >= base + 5000, "expected >= {} got {max}", base + 5000);
}
