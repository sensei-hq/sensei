    //! DB-backed: `resolve_local_pack_raws` folds ADOPTED rule-pack rules into the
    //! local governance ladder (D-LOCAL-PACKS) — the offline half of the two-plane
    //! resolution. Proves the field mapping (statement→title, body→content,
    //! rationale→impact, adoption-namespace scope_key→scope, source→namespace),
    //! never-weaken effective
    //! enforcement (an adoption tier LIFTS a weaker rule but never LOWERS a stronger
    //! one), and that an UN-adopted pack governs nothing. Self-skips when the test DB
    //! is absent, like the neighbouring pg_store tests.
    use super::*;

    #[tokio::test]
    async fn adopted_pack_rules_resolve_with_never_weaken() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let pool = pg.pool();

        // Clean any leftovers from a prior aborted run (slug is globally unique;
        // delete cascades the pack's rules + adoptions).
        for slug in ["pack-resolution-test", "pack-unadopted-test"] {
            sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = $1")
                .bind(slug).execute(pool).await.unwrap();
        }

        // A 'general' scope + namespace: a general/user adoption resolves for ANY folder.
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('general', 'General', 5, false)
             ON CONFLICT (key) DO NOTHING")
            .execute(pool).await.unwrap();
        let ns = pg.upsert_namespace("general", "Bundled", "bundled-test").await.unwrap();

        // Adopted pack: two rules with different default tiers (advisory < required).
        let (pack,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.rule_packs
                (slug, name, area, attribution, summary, enforcement, owner_namespace_id, status, published_by)
             VALUES ('pack-resolution-test', 'T', 'principles', 'TestSource', 's',
                     'recommended', NULL, 'active', 'test')
             RETURNING id")
            .fetch_one(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_rules(pack_id, ordinal, statement, body, rationale, enforcement)
             VALUES ($1, 1, 'S1', 'B1', 'R1', 'advisory'),
                    ($1, 2, 'S2', 'B2', NULL, 'required')")
            .bind(pack).execute(pool).await.unwrap();

        // Un-adopted pack: its rule must never resolve (a pack governs nothing until adopted).
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_packs
                (slug, name, area, attribution, summary, enforcement, owner_namespace_id, status, published_by)
             VALUES ('pack-unadopted-test', 'U', 'security', '', 's', 'mandatory', NULL, 'active', 'test')")
            .execute(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_rules(pack_id, ordinal, statement, body, enforcement)
             SELECT id, 1, 'NOPE', 'B', 'mandatory' FROM sensei.rule_packs WHERE slug='pack-unadopted-test'")
            .execute(pool).await.unwrap();

        // Adopt the first pack at the general namespace with a 'recommended' override.
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_adoptions(pack_id, namespace_id, pinned_version, enforcement, adopted_by)
             VALUES ($1, $2, 1, 'recommended', 'test')")
            .bind(pack).bind(ns).execute(pool).await.unwrap();

        // Resolve for a folder with NO folder_namespaces — only the general clause matches.
        let raws = pg.resolve_local_pack_raws(Some(&uuid::Uuid::new_v4())).await.unwrap();

        let mine: Vec<_> = raws.iter().filter(|r| r.title == "S1" || r.title == "S2").collect();
        assert_eq!(mine.len(), 2, "both adopted-pack rules resolve");
        assert!(!raws.iter().any(|r| r.title == "NOPE"), "an un-adopted pack governs nothing");

        let r1 = raws.iter().find(|r| r.title == "S1").unwrap();
        assert_eq!(r1.content, "B1", "body → content");
        assert_eq!(r1.impact.as_deref(), Some("R1"), "rationale → impact");
        // scope = the GOVERNANCE scope the pack was ADOPTED at (this pack is adopted
        // at the 'general' namespace), NOT the pack's own area/category ('principles').
        // The constitution ladder groups by governance scope, so a rule must carry the
        // scope it entered at — mirrors `resolve_rules_raw` (memories use n.scope_key).
        assert_eq!(r1.scope, "general", "adoption namespace scope_key → scope (not pack area)");
        assert_eq!(r1.namespace.as_deref(), Some("TestSource"), "source → namespace");
        assert_eq!(r1.enforcement, "recommended",
            "an advisory rule is LIFTED to the stronger 'recommended' adoption tier");

        let r2 = raws.iter().find(|r| r.title == "S2").unwrap();
        assert_eq!(r2.enforcement, "required",
            "a 'required' rule is NOT weakened by the lower 'recommended' adoption tier");
        assert_eq!(r2.impact, None, "NULL rationale → None impact");

        // Cleanup (pack delete cascades rules + adoption; then this test's
        // namespace). The shared 'general' scope is left in place — other bundled
        // packs (e.g. the constitution seed) may adopt at it concurrently, so
        // deleting it would FK-fail; in the throwaway test DB a stray scope row
        // is harmless.
        for slug in ["pack-resolution-test", "pack-unadopted-test"] {
            sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = $1")
                .bind(slug).execute(pool).await.unwrap();
        }
        sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = $1").bind(ns).execute(pool).await.unwrap();
    }

    #[tokio::test]
    async fn default_constitution_seed_adopts_offline_but_not_stack_templates() {
        // D-SEED: seed_default_constitution() bundles the constitution as packs
        // and AUTO-ADOPTS the three constitution packs at the general namespace,
        // so a fresh install resolves them offline. The stack-templates pack is
        // seeded but NOT adopted (opt-in per stack).
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let pool = pg.pool();

        // The proc guards on the always-on 'general' scope (seeded by import_scopes
        // in prod); provide it here. Left in place on cleanup (shared).
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('general', 'General', 0, false) ON CONFLICT (key) DO NOTHING")
            .execute(pool).await.unwrap();

        // Fresh from this procedure's definition (idempotent — run twice).
        sqlx_core::query::query("CALL sensei.seed_default_constitution()").execute(pool).await.unwrap();
        sqlx_core::query::query("CALL sensei.seed_default_constitution()").execute(pool).await.unwrap();

        // Four packs; the three constitution packs adopted, stack-templates not.
        let (adopted,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.rule_pack_adoptions a
               JOIN sensei.rule_packs p ON p.id = a.pack_id
               JOIN sensei.namespaces n ON n.id = a.namespace_id
              WHERE n.scope_key='general' AND n.slug='global-dojo'
                AND p.slug IN ('default-principles','default-architecture','default-process')")
            .fetch_one(pool).await.unwrap();
        assert_eq!(adopted, 3, "three constitution packs adopted at general (idempotent — no dup)");

        let raws = pg.resolve_local_pack_raws(Some(&uuid::Uuid::new_v4())).await.unwrap();

        // A mandatory principle resolves, scoped by the ADOPTION (the three
        // constitution packs auto-adopt at the 'general' namespace), NOT the pack area.
        let measure = raws.iter().find(|r| r.title == "Measure, then keep what helps")
            .expect("constitution principle resolves offline");
        assert_eq!(measure.enforcement, "mandatory");
        assert_eq!(measure.scope, "general", "adopted at general → scope 'general' (not pack area)");

        // The 21 adopted constitution rules resolve (4 + 5 + 12); stack templates do not.
        // All three packs are adopted at the SAME 'general' namespace, so every rule
        // now carries scope 'general' (the adoption scope) regardless of its pack area.
        let constitution = raws.iter()
            .filter(|r| r.scope == "general")
            .filter(|r| r.namespace.as_deref() == Some("sensei default constitution (DORA · XP/CD · Core Protocols)"))
            .count();
        assert_eq!(constitution, 21, "all constitution rules resolve at the adoption scope (idempotent re-seed did not duplicate)");
        assert!(!raws.iter().any(|r| r.title.contains("[stack:")),
            "stack-templates is seeded but NOT adopted — its rules must not resolve");

        // Cleanup: delete the four packs (cascade rules + adoptions) + the seeded
        // namespace. Leave the shared 'general' scope.
        sqlx_core::query::query(
            "DELETE FROM sensei.rule_packs
              WHERE owner_namespace_id IS NULL
                AND slug IN ('default-principles','default-architecture','default-process','stack-templates')")
            .execute(pool).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE scope_key='general' AND slug='global-dojo'")
            .execute(pool).await.unwrap();
    }

    #[tokio::test]
    async fn resolve_local_checker_rules_returns_only_checker_backed_rules() {
        // D-CHECKER: resolve_local_checker_rules surfaces ONLY adopted rules with
        // verification='checker' + a checker_ref — a 'review' rule in the same pack
        // must not appear. Uses a general adoption so a random folder resolves it.
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = 'checker-resolve-test'")
            .execute(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('general', 'General', 0, false) ON CONFLICT (key) DO NOTHING")
            .execute(pool).await.unwrap();
        let ns = pg.upsert_namespace("general", "Bundled", "checker-ns-test").await.unwrap();
        let (pack,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.rule_packs
                (slug, name, area, attribution, summary, enforcement, owner_namespace_id, status, published_by)
             VALUES ('checker-resolve-test', 'C', 'tech_stack', 's', 's', 'advisory', NULL, 'active', 'test')
             RETURNING id")
            .fetch_one(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_rules(pack_id, ordinal, statement, body, enforcement, verification, checker_ref)
             VALUES ($1, 1, 'run the linter', 'B', 'advisory', 'checker', 'lint'),
                    ($1, 2, 'a manual rule',  'B', 'advisory', 'review',  NULL)")
            .bind(pack).execute(pool).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.rule_pack_adoptions(pack_id, namespace_id, pinned_version, adopted_by)
             VALUES ($1, $2, 1, 'test')")
            .bind(pack).bind(ns).execute(pool).await.unwrap();

        let rules = pg.resolve_local_checker_rules(&uuid::Uuid::new_v4()).await.unwrap();
        let mine: Vec<_> = rules.iter().filter(|(s, _)| s == "run the linter" || s == "a manual rule").collect();
        assert_eq!(mine.len(), 1, "only the checker-backed rule resolves, not the review rule");
        assert_eq!(mine[0], &("run the linter".to_string(), "lint".to_string()));

        sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = 'checker-resolve-test'")
            .execute(pool).await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = $1").bind(ns).execute(pool).await.unwrap();
    }
