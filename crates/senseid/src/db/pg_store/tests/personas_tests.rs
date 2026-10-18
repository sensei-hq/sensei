use super::*;

// ── Personas (Phase 3) ──────────────────────────────────────────────────

/// Seed a project + repository + one user-scope metric row for `email`.
async fn seed_identity_row(
    s: &PgStore,
    uniq: &uuid::Uuid,
    email: &str,
    value: f64,
) -> (uuid::Uuid, uuid::Uuid) {
    let pid = s.create_project(&format!("_test:persona:{uniq}"), None, None).await.unwrap();
    let rid = crate::tasks::test_support::seed_bare_repository(s, &pid, uniq).await;
    let mid = seed_metric(s, &format!("_test:persona:{uniq}:ftr"), "ComputeFtr", 0, None).await;
    s.upsert_project_metric_repo(&MetricRow {
        metric_id: &mid,
        repository_id: &rid,
        scope: "user",
        identity: Some(email),
        commit_sha: None,
        computed_on: chrono::NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
        grain: "daily",
        value,
        props: &serde_json::json!({}),
        source: "measured",
    })
    .await
    .unwrap();
    (pid, rid)
}

#[tokio::test]
async fn resolution_groups_aliases_without_touching_the_raw_email() {
    // The point of the persona layer: two git addresses that belong to one
    // working identity resolve to ONE persona — while `identity` keeps the
    // raw assertion each row was computed from, so the grouping stays a
    // re-derivation rather than a destructive merge.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let a = format!("a-{uniq}@example.com");
    let b = format!("b-{uniq}@example.com");
    let (pid, _rid) = seed_identity_row(&s, &uniq, &a, 0.5).await;
    let uniq2 = uuid::Uuid::new_v4();
    let (pid2, _r2) = seed_identity_row(&s, &uniq2, &b, 0.75).await;

    let persona = s.upsert_persona(&format!("work-{uniq}"), true).await.unwrap();
    s.link_persona_email(&persona, &a, "git").await.unwrap();
    s.link_persona_email(&persona, &b, "git").await.unwrap();
    s.resolve_persona_ids().await.unwrap();

    let (n, distinct_identities): (i64, i64) = query_as(
        "SELECT count(*)::int8, count(DISTINCT identity)::int8 \
               FROM sensei.repository_metrics WHERE persona_id = $1",
    )
    .bind(persona)
    .fetch_one(s.pool())
    .await
    .unwrap();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;
    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid2, None, &[]).await;
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(persona)
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(n, 2, "both alias rows resolve to the one persona");
    assert_eq!(distinct_identities, 2, "the RAW emails are preserved, not collapsed");
}

#[tokio::test]
async fn an_unrecognised_author_stays_unassigned_rather_than_guessed() {
    // Never-fabricate: an email no persona claims must NOT be folded into the
    // local user's numbers. It stays NULL and is surfaced for assignment.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let stranger = format!("stranger-{uniq}@example.com");
    let (pid, _rid) = seed_identity_row(&s, &uniq, &stranger, 0.9).await;

    s.resolve_persona_ids().await.unwrap();
    let (unresolved,): (i64,) = query_as(
        "SELECT count(*)::int8 FROM sensei.repository_metrics \
              WHERE identity = $1 AND persona_id IS NULL",
    )
    .bind(&stranger)
    .fetch_one(s.pool())
    .await
    .unwrap();
    let pending = s.unassigned_identities().await.unwrap();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;

    assert_eq!(unresolved, 1, "an unclaimed author is left unattributed");
    assert!(
        pending.iter().any(|(e, _)| e == &stranger),
        "and is surfaced for assignment rather than silently ignored"
    );
}

#[tokio::test]
async fn reassigning_an_email_moves_its_rows_on_re_resolution() {
    // The reason `identity` is kept: correcting a persona assignment is a
    // re-run, not a data-loss event. Move the address, re-resolve, and the
    // history follows.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let email = format!("moves-{uniq}@example.com");
    let (pid, _rid) = seed_identity_row(&s, &uniq, &email, 0.5).await;

    let first = s.upsert_persona(&format!("first-{uniq}"), true).await.unwrap();
    let second = s.upsert_persona(&format!("second-{uniq}"), true).await.unwrap();
    s.link_persona_email(&first, &email, "git").await.unwrap();
    s.resolve_persona_ids().await.unwrap();

    // The correction: the address actually belongs to the other identity.
    s.link_persona_email(&second, &email, "git").await.unwrap();
    s.resolve_persona_ids().await.unwrap();

    let (on_second,): (i64,) =
        query_as("SELECT count(*)::int8 FROM sensei.repository_metrics WHERE persona_id = $1")
            .bind(second)
            .fetch_one(s.pool())
            .await
            .unwrap();
    let (on_first,): (i64,) =
        query_as("SELECT count(*)::int8 FROM sensei.repository_metrics WHERE persona_id = $1")
            .bind(first)
            .fetch_one(s.pool())
            .await
            .unwrap();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = ANY($1)")
        .bind(vec![first, second])
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(on_second, 1, "the row followed the reassigned address");
    assert_eq!(on_first, 0, "and no longer counts toward the old persona");
}

#[tokio::test]
async fn two_personas_cannot_share_one_dojo_login() {
    // The privacy boundary made structural. Supabase auto-links identities
    // sharing a verified email and cannot be told not to, so two personas CAN
    // end up pointing at one merged account — this must fail loudly rather
    // than silently file business work under a personal identity.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let a = s.upsert_persona(&format!("pa-{uniq}"), true).await.unwrap();
    let b = s.upsert_persona(&format!("pb-{uniq}"), true).await.unwrap();
    let principal = uuid::Uuid::new_v4();

    sqlx_core::query::query("UPDATE sensei.personas SET principal_id = $2 WHERE id = $1")
        .bind(a)
        .bind(principal)
        .execute(s.pool())
        .await
        .unwrap();
    let clash =
        sqlx_core::query::query("UPDATE sensei.personas SET principal_id = $2 WHERE id = $1")
            .bind(b)
            .bind(principal)
            .execute(s.pool())
            .await;

    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = ANY($1)")
        .bind(vec![a, b])
        .execute(s.pool())
        .await
        .unwrap();

    assert!(clash.is_err(), "a second persona claiming the same login is rejected");
}

#[tokio::test]
async fn a_held_back_scope_cannot_crowd_the_push_window() {
    // Live bug #4, and the fix had NO test: every call site passed both values of
    // `metric_scope`, so the filter was a tautology. Held-back user-scoped rows
    // once filled the window and a pass pushed 66 of 132; had they filled it
    // entirely it would have pushed NOTHING while reporting success.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, rid) = seed_sync_fixture(&s, &uniq).await;
    let key = format!("test/bare-{uniq}");
    let mid = seed_metric(&s, &format!("_test:crowd:{uniq}"), "ComputeFtr", 0, None).await;

    // The pushable row is OLDER, so a limit applied before the scope filter would
    // hand every slot to the newer user-scoped rows and miss it entirely.
    s.upsert_project_metric_repo(&MetricRow {
        metric_id: &mid,
        repository_id: &rid,
        scope: "repo",
        identity: None,
        commit_sha: None,
        computed_on: chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        grain: "daily",
        value: 1.0,
        props: &serde_json::json!({}),
        source: "measured",
    })
    .await
    .unwrap();
    for d in 10..14 {
        s.upsert_project_metric_repo(&MetricRow {
            metric_id: &mid,
            repository_id: &rid,
            scope: "user",
            identity: Some("crowd@example.com"),
            commit_sha: None,
            computed_on: chrono::NaiveDate::from_ymd_opt(2026, 8, d).unwrap(),
            grain: "daily",
            value: 9.0,
            props: &serde_json::json!({}),
            source: "measured",
        })
        .await
        .unwrap();
    }

    let got = s.unpushed_metric_rows(&["repo"], &[&key], 2).await.unwrap();
    let repo_rows = got.iter().filter(|r| r.repo_key == key && r.scope == "repo").count();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;
    assert!(repo_rows >= 1, "the pushable row must survive a window full of held-back rows");
    assert!(got.iter().all(|r| r.scope == "repo"), "no held-back scope may enter the batch");
}

#[tokio::test]
async fn the_allow_list_cannot_be_crowded_out_either() {
    // Same defect one axis over: the plan's allow-list was filtered in Rust AFTER
    // the LIMIT, so 218 of 500 slots went to repositories that can never be in any
    // plan — rows that never drain and so hold the window forever.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, rid) = seed_sync_fixture(&s, &uniq).await;
    let allowed_key = format!("test/bare-{uniq}");
    let mid = seed_metric(&s, &format!("_test:allow:{uniq}"), "ComputeFtr", 0, None).await;
    s.upsert_project_metric_repo(&MetricRow {
        metric_id: &mid,
        repository_id: &rid,
        scope: "repo",
        identity: None,
        commit_sha: None,
        computed_on: chrono::NaiveDate::from_ymd_opt(2019, 1, 1).unwrap(),
        grain: "daily",
        value: 1.0,
        props: &serde_json::json!({}),
        source: "measured",
    })
    .await
    .unwrap();

    // A second shared repository with NEWER rows that the plan does not allow.
    let other_pid =
        s.create_project(&format!("_test:allow:other:{uniq}"), None, None).await.unwrap();
    let other_uniq = uuid::Uuid::new_v4();
    let other_rid =
        crate::tasks::test_support::seed_bare_repository(&s, &other_pid, &other_uniq).await;
    sqlx_core::query::query("UPDATE sensei.repositories SET visibility = 'shared' WHERE id = $1")
        .bind(other_rid)
        .execute(s.pool())
        .await
        .unwrap();
    for d in 20..24 {
        s.upsert_project_metric_repo(&MetricRow {
            metric_id: &mid,
            repository_id: &other_rid,
            scope: "repo",
            identity: None,
            commit_sha: None,
            computed_on: chrono::NaiveDate::from_ymd_opt(2026, 8, d).unwrap(),
            grain: "daily",
            value: 5.0,
            props: &serde_json::json!({}),
            source: "measured",
        })
        .await
        .unwrap();
    }

    let got = s.unpushed_metric_rows(&["repo"], &[&allowed_key], 2).await.unwrap();
    let mine = got.iter().filter(|r| r.repo_key == allowed_key).count();

    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;
    crate::tasks::test_support::cleanup_metrics_fixture(&s, &other_pid, None, &[]).await;
    assert!(mine >= 1, "an allowed repo's row must survive a window full of unallowed ones");
    assert!(got.iter().all(|r| r.repo_key == allowed_key), "only allowed keys may be batched");
}

#[tokio::test]
async fn two_rows_that_differ_in_every_key_field_arrive_different() {
    // The C4-remediation test asserted each field against a fixture whose value
    // WAS the constant a broken projection emits — `assert_eq!(commit_sha, None)`
    // under a fixture seeding NULL proves nothing. A second, contrasting row makes
    // the projection track the ROW instead of a literal.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, rid) = seed_sync_fixture(&s, &uniq).await;
    let key = format!("test/bare-{uniq}");
    let mid = seed_metric(&s, &format!("_test:contrast:{uniq}"), "ComputeFtr", 0, None).await;
    s.upsert_project_metric_repo(&MetricRow {
        metric_id: &mid,
        repository_id: &rid,
        scope: "repo",
        identity: None,
        commit_sha: Some("deadbeef"),
        computed_on: chrono::NaiveDate::from_ymd_opt(2021, 3, 4).unwrap(),
        grain: "session",
        value: 7.5,
        props: &serde_json::json!({ "n": 7 }),
        source: "estimated",
    })
    .await
    .unwrap();

    let rows = s.unpushed_metric_rows(&["repo", "user"], &[&key], 500).await.unwrap();
    let got = rows.iter().find(|r| r.commit_sha.as_deref() == Some("deadbeef")).cloned();
    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;

    let m = got.expect("the contrasting row is queued");
    assert_eq!(m.scope, "repo", "scope tracks the row, not a literal 'user'");
    assert_eq!(m.grain, "session", "grain tracks the row, not a literal 'daily'");
    assert_eq!(m.source, "estimated", "an estimate must not arrive as 'measured'");
    assert_eq!(m.commit_sha.as_deref(), Some("deadbeef"), "a sha must not be nulled out");
    assert_eq!(m.props["n"], 7, "props travels, not an empty object");
}

#[tokio::test]
async fn a_pushable_row_carries_every_field_the_dojo_needs() {
    // Claim C4 was FALSE: the push query selected only
    // (id, repo_key, metric, computed_on, value), but the ingest endpoint keys on
    // (metric, repository, scope, principal, commit_sha, computed_on, grain) and
    // stores props + source. Missing any of them and the receiving side either
    // rejects the row or, worse, defaults it — filing a `user` row as a `repo`
    // one, or `measured` over an `imputed` value.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, _rid) = seed_sync_fixture(&s, &uniq).await;

    let rows = s
        .unpushed_metric_rows(&["repo", "user"], &[&format!("test/bare-{uniq}")], 500)
        .await
        .unwrap();
    let mine = rows.iter().find(|r| r.repo_key == format!("test/bare-{uniq}"));

    let got = mine.cloned();
    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;

    let m = got.expect("the seeded shared repository's row is queued for push");
    assert_eq!(m.scope, "user", "scope round-trips — the fixture seeds a user-scoped row");
    assert_eq!(m.grain, "daily", "grain is part of the dōjō's unique key");
    assert_eq!(m.value, 0.5);
    assert_eq!(m.source, "measured", "source distinguishes measured from imputed");
    assert!(m.props.is_object(), "props travels as an object, not a string");
    assert_eq!(m.commit_sha, None, "absent stays absent — never defaulted to a placeholder");
    assert!(!m.id.is_nil(), "the local row id comes back so shared_at can be marked");
}

#[tokio::test]
async fn setting_a_repository_shared_is_what_opens_gate_1() {
    // The surface that did not exist. `visibility` is private-by-default on
    // purpose — signing in must not start sharing — so SOMETHING has to set it,
    // and until this there was no API, CLI flag or toggle that could. Gate 1 was
    // unreachable: 0 of 67 repositories were shared (daemon-sync.md C3), and the
    // whole push would have moved nothing while reporting success.
    let s = pg_store().await;
    let key = format!("ztest-host/acme/{}", uuid::Uuid::new_v4());
    sqlx_core::query::query(
        "INSERT INTO sensei.repositories(repo_key, name) VALUES($1, 'ztest-repo')",
    )
    .bind(&key)
    .execute(s.pool())
    .await
    .unwrap();

    // Before: private by default, and gate 1 does not yield it.
    let shared_before = s.shared_repositories(500).await.unwrap();
    let updated = s.set_repository_visibility(&key, "shared").await.unwrap();
    let shared_after = s.shared_repositories(500).await.unwrap();
    // Turning it back off must also work — sharing is revocable, or the "you may
    // change it" half of D8 is a lie.
    let revoked = s.set_repository_visibility(&key, "private").await.unwrap();
    let shared_revoked = s.shared_repositories(500).await.unwrap();
    // A value the enum does not have is an error, not a silent no-op that leaves
    // the user believing they shared something.
    let bogus = s.set_repository_visibility(&key, "public").await;
    // An unknown repo_key writes nothing and does not error — the caller can
    // tell "set" from "no such repository" by the row count.
    let unknown = s.set_repository_visibility("ztest-host/never/seen", "shared").await.unwrap();

    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE repo_key = $1")
        .bind(&key)
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(updated, 1, "the visibility is set");
    assert!(!shared_before.iter().any(|r| r.repo_key == key), "private by default");
    assert!(shared_after.iter().any(|r| r.repo_key == key), "gate 1 now yields it");
    assert_eq!(revoked, 1, "and it can be turned back off");
    assert!(!shared_revoked.iter().any(|r| r.repo_key == key), "gate 1 stops yielding it");
    assert!(bogus.is_err(), "a value outside sensei.repo_visibility is refused");
    assert_eq!(unknown, 0, "an unknown repo_key writes nothing and does not error");
}

#[tokio::test]
async fn a_repository_records_the_tenant_it_was_mapped_to() {
    // D2's payoff. The daemon learns the tenant from the dōjō's registration
    // response and has to keep it; without this the mapping is recomputed every
    // cycle and `sensei.repositories.tenant_id` stays permanently NULL.
    let s = pg_store().await;
    let key = format!("ztest-host/acme/{}", uuid::Uuid::new_v4());
    let tenant = uuid::Uuid::new_v4();
    sqlx_core::query::query(
        "INSERT INTO sensei.repositories(repo_key, name) VALUES($1, 'ztest-repo')",
    )
    .bind(&key)
    .execute(s.pool())
    .await
    .unwrap();

    let wrote = s.set_repository_tenant(&key, tenant).await.unwrap();
    let stored: (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
        "SELECT tenant_id FROM sensei.repositories WHERE repo_key = $1",
    )
    .bind(&key)
    .fetch_one(s.pool())
    .await
    .unwrap();
    // Re-running a cycle must not churn `modified_at` on an unchanged mapping.
    let again = s.set_repository_tenant(&key, tenant).await.unwrap();
    // A repo_key this database has never seen is 0 rows, not an error and not a
    // fabricated insert — the dōjō may map repositories another machine shared.
    let unknown = s.set_repository_tenant("ztest-host/never/seen", tenant).await.unwrap();

    sqlx_core::query::query("DELETE FROM sensei.repositories WHERE repo_key = $1")
        .bind(&key)
        .execute(s.pool())
        .await
        .unwrap();

    assert_eq!(wrote, 1, "the mapping is stored");
    assert_eq!(stored.0, Some(tenant), "and it is the tenant the dōjō named");
    assert_eq!(again, 0, "re-storing the same tenant writes nothing");
    assert_eq!(unknown, 0, "an unknown repo_key writes nothing and does not error");
}

#[tokio::test]
async fn signed_in_personas_lists_only_the_ones_that_completed_a_sign_in() {
    // The registry that replaced the withdrawn `sensei.dojo_personas` table
    // (docs/spec/dojo/daemon-sync.md §3).
    //
    // This test USED to seed only `verified_at`, because the spec claimed the
    // label was the Keychain slot. It is not, and the sign-in that proved it also
    // broke this test — correctly. Both fields are now seeded because both are
    // required: `session_slot` names a real Keychain entry, `verified_at` says the
    // OAuth callback completed.
    //
    // A row proves a sign-in HAPPENED, not that its token is still good; the
    // caller still probes the Keychain.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let signed = format!("ztest-signed-{uniq}");
    let never = format!("ztest-never-{uniq}");
    let a = s.upsert_persona(&signed, true).await.unwrap();
    let b = s.upsert_persona(&never, true).await.unwrap();
    sqlx_core::query::query(
        "UPDATE sensei.personas SET verified_at = now(), session_slot = $2 WHERE id = $1",
    )
    .bind(a)
    .bind(&signed)
    .execute(s.pool())
    .await
    .unwrap();

    let listed = s.signed_in_personas().await.unwrap();

    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = ANY($1)")
        .bind(vec![a, b])
        .execute(s.pool())
        .await
        .unwrap();

    // Containment, not equality: this database holds the developer's real
    // personas too, and a test that pins the whole set fails for reasons that
    // have nothing to do with the code.
    assert!(listed.contains(&signed), "a persona that completed a sign-in is listed");
    assert!(!listed.contains(&never), "one that never signed in is not — it has no Keychain slot");
}

#[tokio::test]
async fn the_registry_returns_the_keychain_slot_not_the_relabelled_persona() {
    // THE bug this column exists for, and it was live: a sign-in as `default`
    // leaves the session at `refresh_token.default` but RELABELS the row to the
    // verified GitHub login. Returning the label sent `live_access_token` looking
    // for `refresh_token.sensei-hq-org`, which does not exist — so it reported
    // SignedOut, skipped the persona, and the cycle claimed success having pushed
    // nothing. 0 plan rows, 0 rows shared, `last_ok = true`.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let slot = format!("ztest-slot-{uniq}");
    // The relabelling is the point: label and slot must differ, exactly as they
    // do after a real sign-in.
    let label = format!("ztest-relabelled-{uniq}");
    let id = s.upsert_persona(&label, true).await.unwrap();
    sqlx_core::query::query(
        "UPDATE sensei.personas SET verified_at = now(), session_slot = $2 WHERE id = $1",
    )
    .bind(id)
    .bind(&slot)
    .execute(s.pool())
    .await
    .unwrap();

    let listed = s.signed_in_personas().await.unwrap();

    // A second persona may not claim the same slot — one stored session serving
    // two identities would push one person's metrics under the other's token.
    let other = s.upsert_persona(&format!("ztest-other-{uniq}"), true).await.unwrap();
    let clash = sqlx_core::query::query(
        "UPDATE sensei.personas SET verified_at = now(), session_slot = $2 WHERE id = $1",
    )
    .bind(other)
    .bind(&slot)
    .execute(s.pool())
    .await;

    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = ANY($1)")
        .bind(vec![id, other])
        .execute(s.pool())
        .await
        .unwrap();

    assert!(listed.contains(&slot), "the registry yields the KEYCHAIN SLOT");
    assert!(
        !listed.contains(&label),
        "and never the label — that is the lookup that found no session and silently skipped"
    );
    assert!(clash.is_err(), "two personas cannot share one Keychain slot");
}

#[tokio::test]
async fn all_task_kinds_match_the_database_enum() {
    // Ties the three places a kind must exist into one assertion: the Rust
    // enum, `TaskKind::ALL`, and `sensei.task_execution_kind`.
    //
    // This is the guard the codebase lacked. task_kind was free text until
    // Phase 0, and four kinds had already orphaned their history by being
    // renamed or retired with nothing noticing. Now a half-added kind fails
    // here instead of at an INSERT on a fire-and-forget path that only logs.
    let s = pg_store().await;
    let db: Vec<String> = query_as::<_, (String,)>(
        "SELECT e.enumlabel::text FROM pg_enum e \
               JOIN pg_type t ON t.oid = e.enumtypid \
              WHERE t.typname = 'task_execution_kind'",
    )
    .fetch_all(s.pool())
    .await
    .unwrap()
    .into_iter()
    .map(|(v,)| v)
    .collect();

    // Values kept only to describe historical rows. They have no TaskKind by
    // design — remapping them to a live kind would fabricate history that
    // never happened (compute_metrics SPLIT in two; resolve_edges was retired
    // outright and has no successor at all).
    let retired = [
        "resolve_edges",
        // Retired outright: its `covers` set is now the sensei.doc_coverage view,
        // and its degree refresh is counted from the adjacency detect_communities
        // already builds. No successor kind — the work does not exist any more.
        "build_connections",
        "plan_metric_days",
        "compute_metrics",
        "reconcile_identity",
        "backfill_transcripts",
        "backfill_transcript_file",
    ];

    for k in crate::tasks::TaskKind::ALL {
        let name = k.info().name;
        assert!(
            db.iter().any(|d| d == name),
            "{name} is a live TaskKind but missing from sensei.task_execution_kind — \
                 an execution row for it would fail to INSERT"
        );
    }
    for value in &db {
        if retired.contains(&value.as_str()) {
            continue;
        }
        assert!(
            crate::tasks::TaskKind::ALL.iter().any(|k| k.info().name == value),
            "{value} exists in the DB enum but no TaskKind produces it — \
                 either add the kind or mark the value retired"
        );
    }
}
