use super::*;

// ── Dōjō confidentiality: workspace identifiers (C5) ─────────────────

#[test]
fn repo_tokens_from_remote_parses_ssh_and_https() {
    assert_eq!(
        repo_tokens_from_remote("git@github.com:acme/acme-api.git"),
        vec!["acme-api".to_string(), "acme".to_string()]
    );
    assert_eq!(
        repo_tokens_from_remote("https://github.com/acme/acme-api"),
        vec!["acme-api".to_string(), "acme".to_string()]
    );
    // Host-like and empty segments are skipped.
    assert!(repo_tokens_from_remote("https://example.com").is_empty());
}

#[test]
fn remote_owner_slug_extracts_lowercased_owner() {
    // ssh + https, mixed case → the owner (segment before the repo), lowercased.
    assert_eq!(
        remote_owner_slug("git@github.com:Sensei-HQ/sensei.git").as_deref(),
        Some("sensei-hq")
    );
    assert_eq!(
        remote_owner_slug("https://github.com/Sensei-HQ/sensei").as_deref(),
        Some("sensei-hq")
    );
    assert_eq!(remote_owner_slug("https://gitlab.com/acme/api.git").as_deref(), Some("acme"));
    // No owner segment / unparseable → None.
    assert_eq!(remote_owner_slug("https://example.com"), None);
    assert_eq!(remote_owner_slug(""), None);
}

#[tokio::test]
async fn suggest_binding_infers_from_git_owner_then_stops_once_bound() {
    let s = PgStore::test_store().await;
    let suffix = format!("suggestbind_{}", uuid::Uuid::new_v4());
    let (pid, fid) = create_test_workspace_and_folder(&s, &suffix).await;
    // An owner no other test uses: the database is shared, and a test that
    // connects an "acme" membership concurrently made "no membership yet" false.
    let owner = format!("acme{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    // The workspace's repo is owned by it, in mixed case in the remote.
    sqlx_core::query::query("UPDATE sensei.folders SET remote_urls = $2 WHERE id = $1")
        .bind(fid)
        .bind(serde_json::json!([{
            "name": "origin",
            "url": format!("git@github.com:{}/widget.git", owner.to_uppercase()),
        }]))
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(
        s.workspace_org_owners(&pid).await.unwrap(),
        vec![owner.clone()],
        "owner parsed + lowercased"
    );

    // No membership connected yet → no suggestion.
    assert!(crate::dojo::memberships::suggest_binding(&s, &pid).await.unwrap().is_none());

    // Connect a client membership covering "acme".
    let mid = uuid::Uuid::new_v4();
    s.create_dojo_membership(&NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: format!("github/{owner}"),
        dojo_url: format!("http://localhost:7755/github/{owner}"),
        kind: "client".into(),
        org_slugs: vec![owner.clone()],
        role: "contributor".into(),
        authenticated_via: "device_code".into(),
        attribution_default: "anonymous".into(),
        sync_status: "healthy".into(),
    })
    .await
    .unwrap();

    // Now it suggests that membership, explaining which owner matched.
    let sug =
        crate::dojo::memberships::suggest_binding(&s, &pid).await.unwrap().expect("a suggestion");
    assert_eq!(sug.membership_id, mid);
    assert_eq!(sug.kind, "client");
    assert_eq!(sug.matched_slug, owner);
    assert_eq!(sug.tenant_key, format!("github/{owner}"));

    // Once the workspace is bound, the chip no longer applies.
    assert!(s.bind_workspace_to_dojo(&pid, Some(&mid)).await.unwrap());
    assert!(crate::dojo::memberships::suggest_binding(&s, &pid).await.unwrap().is_none());

    s.bind_workspace_to_dojo(&pid, None).await.unwrap();
    s.delete_dojo_membership(&mid).await.unwrap();
}

#[tokio::test]
async fn workspace_identifiers_gathers_names_paths_repos_and_sessions() {
    let s = pg_store().await;
    let suffix = format!("projident_{}", uuid::Uuid::new_v4());
    let (pid, fid) = create_test_workspace_and_folder(&s, &suffix).await;
    // Set the client + a git remote so the parser has something to chew on.
    sqlx_core::query::query("UPDATE sensei.workspaces SET client = $2 WHERE id = $1")
        .bind(pid)
        .bind("Acme Corp")
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("UPDATE sensei.folders SET remote_urls = $2 WHERE id = $1")
        .bind(fid)
        .bind(serde_json::json!([{ "name": "origin", "url": "git@github.com:acme/acme-api.git" }]))
        .execute(s.pool())
        .await
        .unwrap();
    // A session for the workspace, with a client_session_id.
    let csid = format!("cs-{suffix}");
    s.record_session_event(&csid, &fid, Some(&pid), "claude", true).await.unwrap();

    let ids = s.workspace_identifiers(&pid).await.unwrap();
    assert_eq!(ids.workspace_name.as_deref(), Some(format!("_test:{suffix}").as_str()));
    assert_eq!(ids.client_name.as_deref(), Some("Acme Corp"));
    assert!(
        ids.repo_names.iter().any(|r| r == "acme-api"),
        "repo from remote missing: {:?}",
        ids.repo_names
    );
    assert!(
        ids.repo_names.iter().any(|r| r == "acme"),
        "owner from remote missing: {:?}",
        ids.repo_names
    );
    assert!(
        ids.folder_paths.iter().any(|p| p.contains(&suffix)),
        "folder path missing: {:?}",
        ids.folder_paths
    );
    assert!(
        ids.session_ids.iter().any(|sid| sid == &csid),
        "client_session_id missing: {:?}",
        ids.session_ids
    );
    // The observatory session UUID is also present.
    assert!(ids.session_ids.len() >= 2, "expected uuid + client_session_id: {:?}", ids.session_ids);

    // Cleanup (workspace delete cascades folders/sessions via FKs).
    s.delete_workspace(&pid).await.ok();
}

// ── Dōjō connections tests ────────────────────────────────────────

#[tokio::test]
async fn dojo_membership_crud_and_workspace_binding_roundtrip() {
    let pg = PgStore::test_store().await;
    // Service-assigned membership id (the local PK; workspaces.dojo_id → this).
    let mid = uuid::Uuid::new_v4();
    pg.create_dojo_membership(&NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: "github/acme".into(),
        dojo_url: "http://localhost:7755/github/acme".into(),
        kind: "client".into(),
        org_slugs: vec!["acme".into(), "acme-labs".into()],
        role: "contributor".into(),
        authenticated_via: "device_code".into(),
        attribution_default: "anonymous".into(),
        sync_status: "authenticating".into(),
    })
    .await
    .unwrap();

    // Present in the list with sane defaults + the org_slugs roundtrip.
    let all = pg.list_dojo_memberships().await.unwrap();
    let row = all.iter().find(|m| m.id == mid).expect("membership listed");
    assert_eq!(row.kind, "client");
    assert_eq!(row.org_slugs, vec!["acme".to_string(), "acme-labs".to_string()]);
    assert_eq!(row.last_seq, 0);
    assert!(row.enabled);
    assert!(row.last_heartbeat_at.is_none());

    // org-tagging edit: replace the covered org slugs.
    assert!(pg.set_dojo_membership_orgs(&mid, &["acme".into(), "acme-corp".into()]).await.unwrap());
    assert_eq!(
        pg.get_dojo_membership(&mid).await.unwrap().unwrap().org_slugs,
        vec!["acme".to_string(), "acme-corp".to_string()]
    );
    assert!(
        !pg.set_dojo_membership_orgs(&uuid::Uuid::new_v4(), &[]).await.unwrap(),
        "unknown id → false"
    );

    // sync-status update.
    assert!(pg.set_dojo_sync_status(&mid, "healthy").await.unwrap());
    assert_eq!(pg.get_dojo_membership(&mid).await.unwrap().unwrap().sync_status, "healthy");

    // Bind a workspace → workspaces.dojo_id → appears in the bound-workspaces strip.
    let proj = pg.create_workspace("_test:dojo:bind", None, None).await.unwrap();
    assert!(pg.bind_workspace_to_dojo(&proj, Some(&mid)).await.unwrap());
    let bound = pg.workspaces_bound_to_dojo(&mid).await.unwrap();
    assert!(bound.iter().any(|(id, _)| *id == proj), "bound workspace surfaces");

    // Unbind + cleanup.
    assert!(pg.bind_workspace_to_dojo(&proj, None).await.unwrap());
    assert!(pg.workspaces_bound_to_dojo(&mid).await.unwrap().is_empty());
    pg.delete_workspace(&proj).await.unwrap();

    assert!(pg.delete_dojo_membership(&mid).await.unwrap());
    assert!(pg.get_dojo_membership(&mid).await.unwrap().is_none());
}

#[tokio::test]
async fn collective_preferences_defaults_and_upsert_roundtrip() {
    let pg = PgStore::test_store().await;
    use crate::collective::preferences::{self, CollectivePreferences};

    // Clean slate — the singleton row may linger from a prior run.
    sqlx_core::query::query("DELETE FROM sensei.collective_preferences")
        .execute(pg.pool())
        .await
        .unwrap();

    // Defaults-when-empty: no row → conservative defaults, updated_at None.
    assert!(pg.get_collective_preferences().await.unwrap().is_none());
    let defaults = preferences::get(&pg).await.unwrap();
    assert_eq!(defaults.destination, "none");
    assert_eq!(defaults.cadence, "manual");
    assert_eq!(defaults.attribution_default, "anonymous");
    assert_eq!(defaults.updated_at, None);

    // Upsert a validated body, then read it back.
    let body = serde_json::json!({
        "destination": "both", "cadence": "daily", "attribution_default": "named",
        "categories": { "memory": false, "guard": false }
    });
    let saved =
        preferences::set(&pg, CollectivePreferences::from_request(&body).unwrap()).await.unwrap();
    assert!(saved.updated_at.is_some(), "upsert assigns updated_at");

    let got = preferences::get(&pg).await.unwrap();
    assert_eq!(got.destination, "both");
    assert_eq!(got.cadence, "daily");
    assert_eq!(got.attribution_default, "named");
    assert_eq!(got.categories.get("memory"), Some(&false));
    assert_eq!(got.categories.get("guard"), Some(&false));
    assert_eq!(got.categories.get("pattern"), Some(&true));
    assert!(got.updated_at.is_some());

    // Re-upsert → still exactly one row (singleton), values fully replaced.
    let body2 = serde_json::json!({ "destination": "global", "cadence": "weekly" });
    preferences::set(&pg, CollectivePreferences::from_request(&body2).unwrap()).await.unwrap();
    let (n,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.collective_preferences")
            .fetch_one(pg.pool())
            .await
            .unwrap();
    assert_eq!(n, 1, "singleton table holds exactly one row after re-upsert");
    let got2 = preferences::get(&pg).await.unwrap();
    assert_eq!(got2.destination, "global");
    assert_eq!(
        got2.categories.get("memory"),
        Some(&true),
        "full replace resets toggles to default"
    );

    // Cleanup.
    sqlx_core::query::query("DELETE FROM sensei.collective_preferences")
        .execute(pg.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn upsert_stance_default_and_scoped_roundtrip() {
    let pg = PgStore::test_store().await;
    // Unique key so parallel tests / lingering rows never collide.
    let user = format!("stance-upsert-{}@test.local", uuid::Uuid::new_v4());

    // 1. Default row (namespace_id NULL): insert, then re-resolve returns it
    //    as the "default" source.
    let ua = pg.upsert_stance(&user, None, "run_freely", "private", "quorum").await.unwrap();
    assert!(!ua.is_empty(), "upsert returns an updated_at");
    let r = pg.resolve_stance(&user, None).await.unwrap();
    assert_eq!(
        (r.autonomy.as_str(), r.sharing.as_str(), r.review.as_str(), r.source.as_str()),
        ("run_freely", "private", "quorum", "default")
    );

    // 2. Re-upsert the default (same partial-index conflict target): updates in
    //    place, no duplicate default row.
    pg.upsert_stance(&user, None, "ask_always", "patterns", "me_alone").await.unwrap();
    let r = pg.resolve_stance(&user, None).await.unwrap();
    assert_eq!(
        (r.autonomy.as_str(), r.review.as_str()),
        ("ask_always", "me_alone"),
        "default row updated"
    );
    let (n,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM sensei.stances WHERE user_key = $1 AND namespace_id IS NULL",
    )
    .bind(&user)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "exactly one default row after re-upsert");

    // 3. Scoped row: seed a throwaway namespace, upsert against it (composite
    //    conflict target), read it back, then re-upsert to prove update. The
    //    `workspace` scope may be absent in an unseeded test DB — seed it idempotently.
    sqlx_core::query::query(
        "INSERT INTO sensei.scopes (key, name, level) VALUES ('workspace', 'Workspace', 60)
             ON CONFLICT (key) DO NOTHING",
    )
    .execute(pg.pool())
    .await
    .unwrap();
    let ns = uuid::Uuid::new_v4();
    sqlx_core::query::query(
        "INSERT INTO sensei.namespaces (id, scope_key, name, slug, level)
             VALUES ($1, 'workspace', 'stance-test', $2, 60)",
    )
    .bind(ns)
    .bind(format!("stance-test-{ns}"))
    .execute(pg.pool())
    .await
    .unwrap();

    pg.upsert_stance(&user, Some(&ns), "ask_on_risky", "derived", "two_maintainers").await.unwrap();
    let (au, sh, rv): (String, String, String) = sqlx_core::query_as::query_as(
        "SELECT autonomy::text, sharing::text, review::text FROM sensei.stances
             WHERE user_key = $1 AND namespace_id = $2",
    )
    .bind(&user)
    .bind(ns)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    assert_eq!(
        (au.as_str(), sh.as_str(), rv.as_str()),
        ("ask_on_risky", "derived", "two_maintainers")
    );

    pg.upsert_stance(&user, Some(&ns), "run_freely", "patterns", "quorum").await.unwrap();
    let (au2,): (String,) = sqlx_core::query_as::query_as(
        "SELECT autonomy::text FROM sensei.stances WHERE user_key = $1 AND namespace_id = $2",
    )
    .bind(&user)
    .bind(ns)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    assert_eq!(au2, "run_freely", "scoped row updated via composite conflict target");
    let (n,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.stances WHERE user_key = $1")
            .bind(&user)
            .fetch_one(pg.pool())
            .await
            .unwrap();
    assert_eq!(n, 2, "one default + one scoped row for the user");

    // Cleanup (stances cascade off the namespace delete).
    sqlx_core::query::query("DELETE FROM sensei.stances WHERE user_key = $1")
        .bind(&user)
        .execute(pg.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = $1")
        .bind(ns)
        .execute(pg.pool())
        .await
        .ok();
}

#[tokio::test]
async fn dojo_outbox_and_batch_items_roundtrip() {
    let pg = PgStore::test_store().await;

    // A workspace + a memory to share + an APPROVED batch containing it.
    let proj = pg.create_workspace("_test:dojo:outbox", None, None).await.unwrap();
    let mem = pg
        .insert_memory(&InsertMemory {
            workspace_id: Some(proj),
            scope: "workspace".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "prefer migration tools".into(),
            content: "Use a dedicated migration tool over hand-rolled SQL.".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
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
    let batch = pg.create_memory_share_batch(&proj, &[mem], None).await.unwrap();
    pg.set_memory_share_batch_status(&batch, "approved", None).await.unwrap();

    // batch_share_items: approved batch, one member. The body is
    // `generalised_content` ONLY — this memory has none, so the body is EMPTY
    // rather than the raw `content`.
    //
    // It used to COALESCE to `m.content` and this assertion read
    // `body.contains("migration tool")` — i.e. it asserted that an
    // UN-GENERALISED memory ships its raw text. The row still comes through
    // (an absent row is indistinguishable from an empty batch); the caller
    // refuses it as `held_not_generalised`.
    let (bp, status, items) = pg.batch_share_items(&batch).await.unwrap().expect("batch loads");
    assert_eq!(bp, proj);
    assert_eq!(status, "approved");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].memory_id, mem);
    assert!(
        items[0].body.is_empty(),
        "no generalised_content means NOTHING shareable — the raw memory must not leak in as a fallback, got {:?}",
        items[0].body
    );
    assert_eq!(items[0].memory_type, "convention");

    // An unbound workspace → no routing anchor.
    assert!(pg.workspace_bound_membership(&proj).await.unwrap().is_none());

    // While the member has no `sent` outbox row, an unsent approved batch exists.
    assert!(pg.next_unsent_approved_batch().await.unwrap().is_some());

    // A destination membership + the outbox dedup ledger.
    let mid = uuid::Uuid::new_v4();
    pg.create_dojo_membership(&NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: "github/acme".into(),
        dojo_url: "http://localhost:7755/github/acme".into(),
        kind: "client".into(),
        org_slugs: vec![],
        role: "contributor".into(),
        authenticated_via: "device_code".into(),
        attribution_default: "anonymous".into(),
        sync_status: "healthy".into(),
    })
    .await
    .unwrap();

    assert!(!pg.outbox_already_sent(&mid, "sig-1").await.unwrap());
    pg.outbox_mark_sent(&crate::db::pg_store::OutboxSentRow {
        membership_id: &mid,
        batch_id: Some(&batch),
        memory_id: Some(&mem),
        signature: "sig-1",
        sent_seq: 5,
        remote_id: "remote-1",
    })
    .await
    .unwrap();
    assert!(pg.outbox_already_sent(&mid, "sig-1").await.unwrap());
    // A different signature is independent.
    assert!(!pg.outbox_already_sent(&mid, "sig-2").await.unwrap());
    // A late held/queued signal must NOT downgrade an already-sent row.
    pg.outbox_mark_state(&mid, Some(&batch), Some(&mem), "sig-1", "queued").await.unwrap();
    assert!(
        pg.outbox_already_sent(&mid, "sig-1").await.unwrap(),
        "sent row must survive a late queued mark"
    );
    // A held record for a fresh signature.
    pg.outbox_mark_state(&mid, Some(&batch), Some(&mem), "sig-3", "held").await.unwrap();
    assert!(!pg.outbox_already_sent(&mid, "sig-3").await.unwrap());

    // Cleanup: membership delete cascades its outbox rows; then batch/memory/workspace.
    assert!(pg.delete_dojo_membership(&mid).await.unwrap());
    pg.delete_workspace(&proj).await.unwrap();
}

#[tokio::test]
async fn generalisation_persists_the_example_and_the_share_path_carries_it() {
    let pg = PgStore::test_store().await;

    let proj = pg.create_workspace("_test:memories:example", None, None).await.unwrap();
    let mem = pg
        .insert_memory(&InsertMemory {
            workspace_id: Some(proj),
            scope: "workspace".into(),
            scope_filter: None,
            mtype: "convention".into(),
            title: "green ci before merge".into(),
            // The RAW memory — local reference only. It names a real repo path
            // on purpose: nothing on the share path may ever echo it.
            content: "Never merge acme-api from /Users/dev/work/acme-api on red CI.".into(),
            impact: None,
            tags: vec![],
            triage_signal: None,
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

    // The generalisation writes BOTH the portable rule and its synthetic example.
    pg.set_memory_generalisation(
        mem,
        "Gate merges on a green pipeline.",
        Some("A team merges on a red build before a long weekend and loses Monday to a bisect."),
    )
    .await
    .unwrap()
    .expect("the memory exists");

    let batch = pg.create_memory_share_batch(&proj, &[mem], None).await.unwrap();
    pg.set_memory_share_batch_status(&batch, "approved", None).await.unwrap();
    let (_p, _s, items) = pg.batch_share_items(&batch).await.unwrap().expect("batch loads");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].body, "Gate merges on a green pipeline.");
    assert_eq!(
        items[0].example.as_deref(),
        Some("A team merges on a red build before a long weekend and loses Monday to a bisect."),
        "the synthetic example travels with the shareable body",
    );
    // The raw memory is NOT on this path — neither as the body nor as the example.
    for text in [Some(items[0].body.as_str()), items[0].example.as_deref()].into_iter().flatten() {
        assert!(!text.contains("acme-api"), "raw repo name reached the share path: {text:?}");
        assert!(!text.contains("/Users/dev"), "raw path reached the share path: {text:?}");
    }

    // Re-generalising with NO example clears the stale one rather than pairing a
    // new rule with an old illustration.
    pg.set_memory_generalisation(mem, "Do not merge on red.", None).await.unwrap().unwrap();
    let (_p, _s, items) = pg.batch_share_items(&batch).await.unwrap().expect("batch loads");
    assert_eq!(items[0].body, "Do not merge on red.");
    assert_eq!(items[0].example, None, "an example never outlives the rewrite it illustrates");

    pg.delete_workspace(&proj).await.unwrap();
}

#[tokio::test]
async fn dojo_inbox_upsert_apply_and_state_roundtrip() {
    let pg = PgStore::test_store().await;

    // A source membership (inbox rows cascade with it on delete).
    let mid = uuid::Uuid::new_v4();
    pg.create_dojo_membership(&NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: "github/acme".into(),
        dojo_url: "http://localhost:7755/github/acme".into(),
        kind: "community".into(),
        org_slugs: vec![],
        role: "contributor".into(),
        authenticated_via: "device_code".into(),
        attribution_default: "anonymous".into(),
        sync_status: "healthy".into(),
    })
    .await
    .unwrap();

    let attribution = dojo_protocol::Attribution {
        mode: dojo_protocol::AttributionMode::Anonymous,
        author: None,
        org: None,
        anonymous_id: Some("anon-1".into()),
    };
    let row = |sig: &str, title: &str| crate::collective::inbox::InboxRow {
        membership_id: mid,
        artifact_seq: 3,
        signature: sig.into(),
        remote_id: "art-x".into(),
        kind: "principle".into(),
        title: title.into(),
        body: "keep units testable".into(),
        scope: dojo_protocol::ArtifactScope::default(),
        attribution: attribution.clone(),
    };

    // Upsert is idempotent by (membership, signature): first inserts, re-pull skips.
    assert!(pg.upsert_dojo_inbox(&row("inbox-sig-1", "prefer small fns")).await.unwrap());
    assert!(
        !pg.upsert_dojo_inbox(&row("inbox-sig-1", "prefer small fns")).await.unwrap(),
        "re-pull dedups"
    );
    pg.upsert_dojo_inbox(&row("inbox-sig-2", "write tests first")).await.unwrap();

    let items = pg.list_dojo_inbox(true).await.unwrap();
    let item1 = items
        .iter()
        .find(|i| i.artifact_signature == "inbox-sig-1")
        .expect("row 1 present")
        .clone();
    let id2 =
        items.iter().find(|i| i.artifact_signature == "inbox-sig-2").expect("row 2 present").id;
    assert_eq!(item1.kind, "principle");
    assert_eq!(item1.state, "pending");
    assert_eq!(item1.attribution.anonymous_id.as_deref(), Some("anon-1"));

    // resolve_workspace_by_name — the scope-match lookup.
    let proj = pg.create_workspace("_test:dojo:inbox", None, None).await.unwrap();
    assert_eq!(pg.resolve_workspace_by_name("_test:dojo:inbox".into()).await.unwrap(), Some(proj));
    assert!(
        pg.resolve_workspace_by_name(format!("nope-{}", uuid::Uuid::new_v4()))
            .await
            .unwrap()
            .is_none()
    );

    // Land item1 as a global origin='dojo' memory; the row flips to applied.
    let mem_input = InsertMemory {
        workspace_id: None,
        scope: "global".into(),
        scope_filter: None,
        mtype: "convention".into(),
        title: item1.title.clone(),
        content: item1.body.clone(),
        impact: None,
        tags: vec!["dojo".into()],
        triage_signal: None,
        status: "active".into(),
        namespace_id: None,
        enforcement: Some("recommended".into()),
        origin: Some("dojo".into()),
        source_id: None,
        spine_slot: None,
        feature: None,
    };
    let memory_id = pg.land_dojo_inbox_memory(item1.id, &mem_input).await.unwrap();
    let (origin,): (String,) =
        sqlx_core::query_as::query_as("SELECT origin FROM sensei.memories WHERE id = $1")
            .bind(memory_id)
            .fetch_one(pg.pool())
            .await
            .unwrap();
    assert_eq!(origin, "dojo");
    let applied = pg.get_dojo_inbox(item1.id).await.unwrap().unwrap();
    assert_eq!(applied.state, "applied");
    assert_eq!(applied.applied_memory_id, Some(memory_id));

    // mute hides from the default list; pin floats to the top; unknown → false.
    assert!(pg.set_dojo_inbox_state(id2, "muted").await.unwrap());
    assert!(
        pg.list_dojo_inbox(false).await.unwrap().iter().all(|i| i.id != id2),
        "muted hidden by default"
    );
    assert!(
        pg.list_dojo_inbox(true).await.unwrap().iter().any(|i| i.id == id2),
        "include_muted surfaces it"
    );
    assert!(pg.set_dojo_inbox_state(id2, "pinned").await.unwrap());
    assert_eq!(pg.list_dojo_inbox(false).await.unwrap()[0].id, id2, "pinned floats to the top");
    assert!(
        !pg.set_dojo_inbox_state(uuid::Uuid::new_v4(), "muted").await.unwrap(),
        "unknown id → false"
    );

    // Cursor advance lives on the membership's last_seq.
    pg.set_dojo_pull_cursor(mid, 42).await.unwrap();
    assert_eq!(pg.get_dojo_membership(&mid).await.unwrap().unwrap().last_seq, 42);

    // Cleanup: membership delete cascades inbox rows; then memory + workspace.
    assert!(pg.delete_dojo_membership(&mid).await.unwrap());
    sqlx_core::query::query("DELETE FROM sensei.memories WHERE id = $1")
        .bind(memory_id)
        .execute(pg.pool())
        .await
        .unwrap();
    pg.delete_workspace(&proj).await.unwrap();
}

// ── Sync bookkeeping (Phase 7) ──────────────────────────────────────────

/// A shared repository with one locally-computed metric row.
pub(super) async fn seed_sync_fixture(s: &PgStore, uniq: &uuid::Uuid) -> (uuid::Uuid, uuid::Uuid) {
    seed_sync_fixture_at(s, uniq, "shared").await
}

/// The same, at a chosen local `sensei.repositories.visibility`. `'private'` is the
/// org-mandate shape: nothing elected locally, and the dōjō's plan deciding anyway.
pub(super) async fn seed_sync_fixture_at(
    s: &PgStore,
    uniq: &uuid::Uuid,
    visibility: &str,
) -> (uuid::Uuid, uuid::Uuid) {
    let pid = s.create_workspace(&format!("_test:sync:{uniq}"), None, None).await.unwrap();
    let rid = crate::tasks::test_support::seed_bare_repository(s, &pid, uniq).await;
    sqlx_core::query::query(
        "UPDATE sensei.repositories SET visibility = $2::sensei.repo_visibility WHERE id = $1",
    )
    .bind(rid)
    .bind(visibility)
    .execute(s.pool())
    .await
    .unwrap();
    let mid = seed_metric(s, &format!("_test:sync:{uniq}:ftr"), "ComputeFtr", 0, None).await;
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: chrono::NaiveDate::from_ymd_opt(2020, 5, 1).unwrap(),
        grain: "daily",
        value: 0.5,
        props: &serde_json::json!({}),
        source: "measured",
    })
    .await
    .unwrap();
    (pid, rid)
}

#[tokio::test]
async fn a_pulled_row_is_never_pushed_back() {
    // The loop-breaker. Without `computed_by`, a value dojo handed down is
    // indistinguishable from one this machine produced — so it gets pushed
    // back, pulled again, and the two sides ping-pong forever.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, rid) = seed_sync_fixture(&s, &uniq).await;

    let mine = s
        .unpushed_metric_rows(&["repo", "user"], &[&format!("test/bare-{uniq}")], 100)
        .await
        .unwrap();
    let before = mine.iter().filter(|r| r.repo_key == format!("test/bare-{uniq}")).count();

    // Same row, but marked as dojo's.
    sqlx_core::query::query(
        "UPDATE sensei.repository_metrics SET computed_by = 'dojo' WHERE repository_id = $1",
    )
    .bind(rid)
    .execute(s.pool())
    .await
    .unwrap();
    let after = s
        .unpushed_metric_rows(&["repo", "user"], &[&format!("test/bare-{uniq}")], 100)
        .await
        .unwrap();
    let after_n = after.iter().filter(|r| r.repo_key == format!("test/bare-{uniq}")).count();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;

    assert_eq!(before, 1, "a locally-computed row is queued for push");
    assert_eq!(after_n, 0, "a dojo-computed row is NOT pushed back");
}

#[tokio::test]
async fn a_locally_private_repository_is_queued_only_when_the_plan_names_it() {
    // Was `a_private_repository_is_skipped_not_queued`, which asserted that the
    // LOCAL flag did the excluding. §8a's B2 moved that, and the assertion is
    // rescoped rather than dropped because its concern survives: private is a
    // choice, not a backlog item, and a repository nobody elected still never
    // enters the queue — it is simply not in the dōjō's allow-list that keeps it
    // out now, rather than a column the mandate cannot see.
    //
    // Both directions on the SAME private fixture, so it cannot pass by the local
    // flag quietly doing the work again.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, rid) = seed_sync_fixture(&s, &uniq).await;
    let key = format!("test/bare-{uniq}");
    sqlx_core::query::query("UPDATE sensei.repositories SET visibility = 'private' WHERE id = $1")
        .bind(rid)
        .execute(s.pool())
        .await
        .unwrap();

    let omitted = s.unpushed_metric_rows(&["repo", "user"], &[], 100).await.unwrap();
    let named = s.unpushed_metric_rows(&["repo", "user"], &[&key], 100).await.unwrap();
    let omitted_n = omitted.iter().filter(|r| r.repo_key == key).count();
    let named_n = named.iter().filter(|r| r.repo_key == key).count();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;
    assert_eq!(omitted_n, 0, "a repository the plan does not name is never queued for push");
    assert_eq!(
        named_n, 1,
        "and one the plan DOES name is queued despite `visibility = 'private'` — the org \
         mandate is a dōjō decision with no local column to read"
    );
}

#[tokio::test]
async fn a_recomputed_row_is_pushed_again() {
    // shared_at alone is not enough: a day that recomputes after being pushed
    // must go again, or dojo keeps a stale value forever behind an
    // already-synced marker.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, rid) = seed_sync_fixture(&s, &uniq).await;
    let key = format!("test/bare-{uniq}");

    let ids: Vec<uuid::Uuid> = s
        .unpushed_metric_rows(&["repo", "user"], &[&format!("test/bare-{uniq}")], 100)
        .await
        .unwrap()
        .iter()
        .filter(|r| r.repo_key == key)
        .map(|r| r.id)
        .collect();
    s.mark_metric_rows_shared(&ids).await.unwrap();
    let after_push = s
        .unpushed_metric_rows(&["repo", "user"], &[&format!("test/bare-{uniq}")], 100)
        .await
        .unwrap()
        .iter()
        .filter(|r| r.repo_key == key)
        .count();

    // The day recomputes — modified_at moves past shared_at.
    sqlx_core::query::query(
            "UPDATE sensei.repository_metrics SET modified_at = now() + interval '1 second'               WHERE repository_id = $1")
            .bind(rid).execute(s.pool()).await.unwrap();
    let after_recompute = s
        .unpushed_metric_rows(&["repo", "user"], &[&format!("test/bare-{uniq}")], 100)
        .await
        .unwrap()
        .iter()
        .filter(|r| r.repo_key == key)
        .count();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;
    assert_eq!(after_push, 0, "a pushed row leaves the queue");
    assert_eq!(after_recompute, 1, "and re-enters it when the value changes");
}

/// #307 A2d: the forge mirror's cadence keys on the last ATTEMPT, so a mirror
/// that keeps failing waits its interval instead of re-reading GitHub every cycle.
#[tokio::test]
async fn the_last_attempt_is_read_back_for_a_mark_and_absent_before_any() {
    let s = pg_store().await;
    let key = format!("persona-{}", uuid::Uuid::new_v4());
    let mark = crate::db::pg_store::sync::SyncMark {
        entity: "forge_mirror",
        key: &key,
        direction: "pull",
    };

    let before = s.sync_attempted_at(&mark).await.unwrap();
    s.mark_sync_error(&mark, "GitHub read failed (502)").await.unwrap();
    let after = s.sync_attempted_at(&mark).await.unwrap();

    sqlx_core::query::query("DELETE FROM sensei.sync_state WHERE entity_key = $1")
        .bind(&key)
        .execute(s.pool())
        .await
        .unwrap();

    assert!(before.is_none(), "never attempted");
    assert!(after.is_some(), "a failed attempt is still an attempt");
}

#[tokio::test]
async fn a_sync_error_keeps_when_the_sides_last_agreed() {
    // synced_at must survive a failure. Clearing it would leave no way to tell
    // a never-synced entity from one that has been broken since Tuesday —
    // which is the first thing worth knowing when sync starts failing.
    let s = pg_store().await;
    let key = format!("github.com/test/sync-{}", uuid::Uuid::new_v4());
    let mark =
        crate::db::pg_store::sync::SyncMark { entity: "repository", key: &key, direction: "push" };

    s.mark_synced(&mark, Some(7)).await.unwrap();
    s.mark_sync_error(&mark, "boom").await.unwrap();

    let (state, last_error, synced_at): (String, Option<String>, Option<chrono::DateTime<chrono::Utc>>) =
            query_as("SELECT state, last_error, synced_at FROM sensei.sync_state                        WHERE entity = 'repository' AND entity_key = $1 AND direction = 'push'")
                .bind(&key).fetch_one(s.pool()).await.unwrap();

    sqlx_core::query::query("DELETE FROM sensei.sync_state WHERE entity_key = $1")
        .bind(&key)
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(state, "error");
    assert_eq!(last_error.as_deref(), Some("boom"));
    assert!(synced_at.is_some(), "the last agreement time survives a later failure");
}

/// A push that landed all but a named part is `partial` (#225): the sides DID
/// agree, so `synced_at` is set, and the detail says what is missing. `error`
/// would read "failing · never agreed" over a graph that is 1,997 of 1,998 there.
#[tokio::test]
async fn a_partial_sync_agrees_and_names_what_is_missing() {
    let s = pg_store().await;
    let key = format!("github.com/test/partial-{}@main", uuid::Uuid::new_v4());
    let mark = crate::db::pg_store::sync::SyncMark {
        entity: "repository_graph",
        key: &key,
        direction: "push",
    };

    s.mark_sync_partial(&mark, "1 file(s) over the cap, not shared: big.rs").await.unwrap();
    let (state, detail, synced_at): (
        String,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
    ) = query_as(
        "SELECT state, last_error, synced_at FROM sensei.sync_state WHERE entity_key = $1",
    )
    .bind(&key)
    .fetch_one(s.pool())
    .await
    .unwrap();
    // A whole push afterwards clears the detail, as any success does.
    s.mark_synced(&mark, None).await.unwrap();
    let (after,): (Option<String>,) =
        query_as("SELECT last_error FROM sensei.sync_state WHERE entity_key = $1")
            .bind(&key)
            .fetch_one(s.pool())
            .await
            .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.sync_state WHERE entity_key = $1")
        .bind(&key)
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(state, "partial");
    assert_eq!(detail.as_deref(), Some("1 file(s) over the cap, not shared: big.rs"));
    assert!(synced_at.is_some(), "a partial push is an agreement");
    assert_eq!(after, None, "a later whole push clears what was missing");
}

/// The reader ranks `partial` after `pending` and before `skipped`: it needs a
/// look, a skip is a settled decision.
#[tokio::test]
async fn sync_state_rows_rank_partial_between_pending_and_skipped() {
    let s = pg_store().await;
    let tag = uuid::Uuid::new_v4();
    let keys: Vec<String> = ["skip", "part", "pend"]
        .iter()
        .map(|k| format!("github.com/test/rank-{k}-{tag}"))
        .collect();
    fn mark(k: &str) -> crate::db::pg_store::sync::SyncMark<'_> {
        crate::db::pg_store::sync::SyncMark { entity: "repository", key: k, direction: "push" }
    }
    s.mark_sync_skipped(&mark(&keys[0]), "private").await.unwrap();
    s.mark_sync_partial(&mark(&keys[1]), "part missing").await.unwrap();
    sqlx_core::query::query(
        "INSERT INTO sensei.sync_state (entity, entity_key, direction, state) VALUES ('repository', $1, 'push', 'pending')",
    )
    .bind(&keys[2])
    .execute(s.pool())
    .await
    .unwrap();

    let order: Vec<String> = s
        .sync_state_rows()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| keys.contains(&r.entity_key))
        .map(|r| r.state)
        .collect();
    sqlx_core::query::query("DELETE FROM sensei.sync_state WHERE entity_key = ANY($1)")
        .bind(&keys)
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(order, vec!["pending", "partial", "skipped"]);
}

#[tokio::test]
async fn a_verified_login_replaces_a_guessed_label_but_not_a_chosen_one() {
    // The `sensei-hq` vs `sensei-hq-org` case. Before OAuth a label can only
    // be inferred from an email domain or a repo owner, and inference is
    // wrong; after OAuth the login is known. But once a user has verified and
    // then renamed, a later sign-in must not silently overwrite their choice.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let gh_id: i64 = (uniq.as_u128() % 1_000_000) as i64 + 900_000;
    let guessed = format!("guess-{uniq}");

    // Discovered from git: a guessed label, unverified.
    let pid = s.upsert_persona(&guessed, true).await.unwrap();
    let id = s.link_persona_identity(&guessed, "real-login", gh_id, None, &[]).await.unwrap();
    assert_eq!(id, pid, "verification lands on the existing persona");

    let (label, login, verified): (String, Option<String>, Option<chrono::DateTime<chrono::Utc>>) =
        query_as("SELECT label, github_login, verified_at FROM sensei.personas WHERE id = $1")
            .bind(id)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(label, "real-login", "the guess is replaced by the verified login");
    assert_eq!(login.as_deref(), Some("real-login"));
    assert!(verified.is_some());

    // The user renames it, then signs in again.
    sqlx_core::query::query("UPDATE sensei.personas SET label = 'my-name' WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
    s.link_persona_identity("ignored", "real-login", gh_id, None, &[]).await.unwrap();
    let (after,): (String,) = query_as("SELECT label FROM sensei.personas WHERE id = $1")
        .bind(id)
        .fetch_one(s.pool())
        .await
        .unwrap();

    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(after, "my-name", "a chosen display name survives re-verification");
}

#[tokio::test]
async fn a_renamed_github_login_still_matches_the_same_persona() {
    // Matching on the login would fork one human into two personas the day
    // they rename themselves. The numeric id cannot be renamed, so it wins.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let gh_id: i64 = (uniq.as_u128() % 1_000_000) as i64 + 800_000;

    let first =
        s.link_persona_identity(&format!("a-{uniq}"), "old-login", gh_id, None, &[]).await.unwrap();
    let second =
        s.link_persona_identity(&format!("b-{uniq}"), "new-login", gh_id, None, &[]).await.unwrap();

    let (login,): (Option<String>,) =
        query_as("SELECT github_login FROM sensei.personas WHERE id = $1")
            .bind(first)
            .fetch_one(s.pool())
            .await
            .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = ANY($1)")
        .bind(vec![first, second])
        .execute(s.pool())
        .await
        .ok();

    assert_eq!(first, second, "the same GitHub id resolves to one persona");
    assert_eq!(login.as_deref(), Some("new-login"), "and the login is updated in place");
}

// ── The OFFER set, and gate 1 · intent ──────────────────────────────────────
//
// Two different questions, and they stopped having the same answer in
// docs/spec/dojo/daemon-sync.md §8a:
//
// * `offerable_repositories` — WHAT THIS MACHINE DISCLOSES. Every locally-scanned
//   repository with a `repo_key`, i.e. every repository the user CLONED. Never a
//   forge listing: membership of an org whose code you have not cloned discloses
//   nothing, because there is nothing on the machine to measure.
// * `shared_repositories` — GATE 1, user intent. `sensei.repositories.visibility`,
//   the FIRST of the three gates (docs/spec/dojo/dojo-auth-provisioning.md §V.3)
//   and the only one the daemon owns.
//
// Gate 1 used to filter the OFFER, which made an organization's mandate over its
// own private code structurally unreachable — the daemon has no local fact that
// says "this one is mandated", so filtering before asking meant never asking. It
// now filters the PUSH, and only for repositories where the USER holds authority.
// Cost (forge visibility) and entitlement (claim/billing/seat) are the dōjō's and
// are never mirrored here.

#[tokio::test]
async fn every_cloned_repository_is_offered_even_when_the_user_has_not_elected_it() {
    // B1. A new employee whose only repository is the org-mandated private one has
    // nothing locally elected. Filtering the offer on gate 1 meant the cycle
    // returned before `register_repositories`, before `sync_plan`, before any push
    // — the feature defeated for exactly the population it exists to serve.
    //
    // The bound that makes disclosing an unelected repository acceptable is the
    // CLONE, not the forge: `sensei.repositories` is populated by the scanner, so
    // this set is what the user actually works on, never an inventory of what they
    // can reach.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_workspace(&format!("_test:offer:{uniq}"), None, None).await.unwrap();

    // elected locally
    let elected = crate::tasks::test_support::seed_bare_repository(&s, &pid, &uniq).await;
    sqlx_core::query::query("UPDATE sensei.repositories SET visibility = 'shared' WHERE id = $1")
        .bind(elected)
        .execute(s.pool())
        .await
        .unwrap();

    // cloned, NOT elected — the org-mandate candidate. Only the dōjō can know.
    let unelected_uniq = uuid::Uuid::new_v4();
    let unelected =
        crate::tasks::test_support::seed_bare_repository(&s, &pid, &unelected_uniq).await;

    // cloned but LOCAL-ONLY (no remote → no repo_key). Still withheld: it has no
    // cross-install identity, so the dōjō would have nothing to map it to.
    let (localonly,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories(repo_key, name, visibility) \
         VALUES(NULL, $1, 'shared') RETURNING id",
    )
    .bind(format!("offer-localonly-{uniq}"))
    .fetch_one(s.pool())
    .await
    .unwrap();

    // A limit past the whole table: the shared test database carries thousands of
    // leftover fixtures, and a 500-row window would answer "absent" for a reason
    // that has nothing to do with the predicate under test.
    const ALL: i64 = 1_000_000;
    let offered = s.offerable_repositories(ALL).await.unwrap();
    let offered_keys: std::collections::HashSet<String> =
        offered.iter().map(|r| r.repo_key.clone()).collect();
    let offered_localonly =
        offered.iter().any(|r| r.name.starts_with(&format!("offer-localonly-{uniq}")));
    let elected_keys: std::collections::HashSet<String> =
        s.shared_repositories(ALL).await.unwrap().into_iter().map(|r| r.repo_key).collect();

    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = ANY($1)")
        .bind(vec![elected, unelected, localonly])
        .execute(s.pool())
        .await
        .ok();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();

    assert!(
        offered_keys.contains(&format!("test/bare-{uniq}")),
        "an elected repository is offered"
    );
    assert!(
        offered_keys.contains(&format!("test/bare-{unelected_uniq}")),
        "a CLONED but unelected repository must still be OFFERED — the daemon cannot \
         tell whether the org mandated it, so refusing to ask is refusing the mandate"
    );
    assert!(
        !offered_localonly,
        "a local-only repository (NULL repo_key) has no cross-install identity and \
         is offered by neither set"
    );
    // And the two sets are genuinely different — gate 1 did not simply move.
    assert!(
        elected_keys.contains(&format!("test/bare-{uniq}")),
        "gate 1 still yields what the user elected"
    );
    assert!(
        !elected_keys.contains(&format!("test/bare-{unelected_uniq}")),
        "gate 1 still withholds the unelected repository from the ELECTED set"
    );
}

#[tokio::test]
async fn the_offer_window_is_a_throughput_bound_not_an_alphabetical_ceiling() {
    // Widening the offer set from "what the user elected" to "every clone" makes the
    // caller's LIMIT ~40x more load-bearing. Ordered by `repo_key`, an install with
    // more repositories than the limit offers the alphabetically-first N forever —
    // so a freshly cloned org-mandated repository sorting after them is never
    // registered, never in the dōjō's plan, and never pushed. That is B1 again, one
    // layer down, and invisible on the 67-repository install it was measured on.
    //
    // The ordering is asserted RELATIVELY (both fixtures dated into the future) so
    // the test does not depend on how many rows the shared test database carries.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let last_key = format!("zzz-sorts-last/{uniq}");
    let first_key = format!("aaa-sorts-first/{uniq}");

    // Cloned, never registered, sorts LAST alphabetically.
    let (unmapped,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories(repo_key, name, modified_at) \
         VALUES($1, 'ztest-unmapped', now() + interval '10 years') RETURNING id",
    )
    .bind(&last_key)
    .fetch_one(s.pool())
    .await
    .unwrap();
    // Already mapped to a tenant, sorts FIRST alphabetically. It is already in the
    // dōjō's answer whether or not it is re-offered, so it must yield the slot.
    let (mapped,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories(repo_key, name, tenant_id, modified_at) \
         VALUES($1, 'ztest-mapped', gen_random_uuid(), now() + interval '9 years') RETURNING id",
    )
    .bind(&first_key)
    .fetch_one(s.pool())
    .await
    .unwrap();

    let window = s.offerable_repositories(1).await.unwrap();

    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = ANY($1)")
        .bind(vec![unmapped, mapped])
        .execute(s.pool())
        .await
        .ok();

    assert_eq!(
        window.first().map(|r| r.repo_key.as_str()),
        Some(last_key.as_str()),
        "the unregistered repository takes the single slot even though it sorts last — \
         registration is what creates the dōjō row the plan is computed from"
    );
}

#[tokio::test]
async fn shared_repositories_returns_only_opted_in_repos_with_a_remote() {
    // GATE 1, and only gate 1. This test used to assert that an unelected repository
    // "must NEVER be offered"; §8a narrowed that, so the claim is now scoped to what
    // survives: gate 1 governs the PUSH for repositories where the USER holds
    // authority. What this machine DISCLOSES is the wider clone-bounded set —
    // asserted next door in
    // `every_cloned_repository_is_offered_even_when_the_user_has_not_elected_it`.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_workspace(&format!("_test:shared:{uniq}"), None, None).await.unwrap();

    // opted in, has a remote → in the elected set
    let shared = crate::tasks::test_support::seed_bare_repository(&s, &pid, &uniq).await;
    sqlx_core::query::query("UPDATE sensei.repositories SET visibility = 'shared' WHERE id = $1")
        .bind(shared)
        .execute(s.pool())
        .await
        .unwrap();

    // NOT opted in → not elected. Private is a choice, not a failure: signing in
    // must never start sharing a repo the user did not choose to share.
    let private_uniq = uuid::Uuid::new_v4();
    let private = crate::tasks::test_support::seed_bare_repository(&s, &pid, &private_uniq).await;

    // opted in but LOCAL-ONLY (no remote → no repo_key) → withheld by BOTH sets. A
    // NULL repo_key is the DDL's marker for "never federated"; it has no
    // cross-install identity, so the dōjō would have nothing to map it to.
    let (localonly,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.repositories(repo_key, name, visibility) \
         VALUES(NULL, $1, 'shared') RETURNING id",
    )
    .bind(format!("localonly-{uniq}"))
    .fetch_one(s.pool())
    .await
    .unwrap();

    let elected = s.shared_repositories(1_000_000).await.unwrap();
    let keys: std::collections::HashSet<&str> =
        elected.iter().map(|r| r.repo_key.as_str()).collect();

    assert!(
        keys.contains(&format!("test/bare-{uniq}").as_str()),
        "an opted-in repository with a remote is elected"
    );
    assert!(
        !keys.contains(&format!("test/bare-{private_uniq}").as_str()),
        "a repository the user did not share is NOT ELECTED, so gate 1 still stops its \
         rows being pushed on the user's own authority. It IS still disclosed — the \
         dōjō is the only party that can tell a mandate from a private repository"
    );
    assert!(
        elected.iter().all(|r| !r.name.starts_with(&format!("localonly-{uniq}"))),
        "a local-only repository (NULL repo_key) has no cross-install identity and \
         is in neither set"
    );

    // cleanup
    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE id = ANY($1)")
        .bind(vec![shared, private, localonly])
        .execute(s.pool())
        .await
        .ok();
}
