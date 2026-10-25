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
    let pg = PgStore::test_store().await;
    let pool = pg.pool();

    // Clean any leftovers from a prior aborted run (slug is globally unique;
    // delete cascades the pack's rules + adoptions).
    for slug in ["pack-resolution-test", "pack-unadopted-test"] {
        sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = $1")
            .bind(slug)
            .execute(pool)
            .await
            .unwrap();
    }

    // A 'general' scope + namespace: a general/user adoption resolves for ANY folder.
    sqlx_core::query::query(
        "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('general', 'General', 5, false)
             ON CONFLICT (key) DO NOTHING",
    )
    .execute(pool)
    .await
    .unwrap();
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

    // Resolve for a folder with NO namespaces of its own — only the general clause matches.
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
    assert_eq!(
        r1.enforcement, "recommended",
        "an advisory rule is LIFTED to the stronger 'recommended' adoption tier"
    );

    let r2 = raws.iter().find(|r| r.title == "S2").unwrap();
    assert_eq!(
        r2.enforcement, "required",
        "a 'required' rule is NOT weakened by the lower 'recommended' adoption tier"
    );
    assert_eq!(r2.impact, None, "NULL rationale → None impact");

    // Cleanup (pack delete cascades rules + adoption; then this test's
    // namespace). The shared 'general' scope is left in place — other bundled
    // packs (e.g. the constitution seed) may adopt at it concurrently, so
    // deleting it would FK-fail; in the throwaway test DB a stray scope row
    // is harmless.
    for slug in ["pack-resolution-test", "pack-unadopted-test"] {
        sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = $1")
            .bind(slug)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = $1")
        .bind(ns)
        .execute(pool)
        .await
        .unwrap();
}

/// Load a jsonl datafile into its staging table, replacing whatever was there.
///
/// Mirrors what dbd's import phase does (`truncate: true`, then load), so the
/// procedures under test see exactly the rows a real deploy would give them.
async fn load_staging(pool: &sqlx_postgres::PgPool, table: &str, jsonl: &str) {
    sqlx_core::query::query(&format!("DELETE FROM staging.{table}")).execute(pool).await.unwrap();
    for line in jsonl.lines().filter(|l| !l.trim().is_empty()) {
        sqlx_core::query::query(&format!(
            "INSERT INTO staging.{table} SELECT * FROM \
             json_populate_record(null::staging.{table}, $1::json)"
        ))
        .bind(line)
        .execute(pool)
        .await
        .unwrap_or_else(|e| panic!("staging.{table} load failed: {e}\nrow: {line}"));
    }
}

#[tokio::test]
async fn default_constitution_import_adopts_offline_but_not_stack_templates() {
    // D-SEED: the constitution ships as packs in the staging datafiles and is
    // AUTO-ADOPTED at the general namespace, so a fresh install resolves it
    // offline. The stack-templates pack is seeded but NOT adopted (opt-in per
    // stack) — it is simply absent from rule_pack_adoptions.jsonl.
    let pg = PgStore::test_store().await;
    let pool = pg.pool();

    // The always-on 'general' scope (seeded by import_scopes in prod); provide it
    // here. Left in place on cleanup (shared).
    sqlx_core::query::query(
        "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('general', 'General', 0, false) ON CONFLICT (key) DO NOTHING",
    )
    .execute(pool)
    .await
    .unwrap();

    // Load the REAL datafiles, not a fixture. The content used to live inside
    // seed_default_constitution(), so this test read the procedure's own text
    // back; now the shipped jsonl is the thing under test, and a rule dropped
    // from it fails here rather than at a user's install.
    load_staging(
        pool,
        "rule_packs",
        include_str!("../../../../../database/import/staging/rule_packs.jsonl"),
    )
    .await;
    load_staging(
        pool,
        "rule_pack_rules",
        include_str!("../../../../../database/import/staging/rule_pack_rules.jsonl"),
    )
    .await;
    load_staging(
        pool,
        "rule_pack_adoptions",
        include_str!("../../../../../database/import/staging/rule_pack_adoptions.jsonl"),
    )
    .await;

    // import_rule_packs drives rules and adoptions itself — dbd does not order
    // imports, so the dependency is expressed in SQL. Run twice: it must be
    // idempotent, which the old procedure's `on conflict do nothing` also gave.
    for _ in 0..2 {
        sqlx_core::query::query("CALL staging.import_rule_packs()").execute(pool).await.unwrap();
    }

    // Four packs; the three constitution packs adopted, stack-templates not.
    let (adopted,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM sensei.rule_pack_adoptions a
               JOIN sensei.rule_packs p ON p.id = a.pack_id
               JOIN sensei.namespaces n ON n.id = a.namespace_id
              WHERE n.scope_key='general' AND n.slug='global-dojo'
                AND p.slug IN ('default-principles','default-architecture','default-process')",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(adopted, 3, "three constitution packs adopted at general (idempotent — no dup)");

    let raws = pg.resolve_local_pack_raws(Some(&uuid::Uuid::new_v4())).await.unwrap();

    // A mandatory principle resolves, scoped by the ADOPTION (the three
    // constitution packs auto-adopt at the 'general' namespace), NOT the pack area.
    let measure = raws
        .iter()
        .find(|r| r.title == "Measure, then keep what helps")
        .expect("constitution principle resolves offline");
    assert_eq!(measure.enforcement, "mandatory");
    assert_eq!(measure.scope, "general", "adopted at general → scope 'general' (not pack area)");

    // The 21 adopted constitution rules resolve (4 + 5 + 12); stack templates do not.
    // All three packs are adopted at the SAME 'general' namespace, so every rule
    // now carries scope 'general' (the adoption scope) regardless of its pack area.
    let constitution = raws
        .iter()
        .filter(|r| r.scope == "general")
        .filter(|r| {
            r.namespace.as_deref()
                == Some("sensei default constitution (DORA · XP/CD · Core Protocols)")
        })
        .count();
    assert_eq!(
        constitution, 21,
        "all constitution rules resolve at the adoption scope (idempotent re-seed did not duplicate)"
    );
    assert!(
        !raws.iter().any(|r| r.title.contains("[stack:")),
        "stack-templates is seeded but NOT adopted — its rules must not resolve"
    );

    // NO CLEANUP, deliberately — and this is a fix, not an omission (#217).
    //
    // This test used to delete the four packs and the `global-dojo` namespace on
    // the way out. They are not rows it created: `dbd deploy` installs exactly
    // the same ones from exactly the same datafiles, so the delete left the
    // SHARED test database (#183) permanently short of its governance seed.
    // Measured 2026-10-05 on a database that had run the suite: 10 rule_packs
    // and 46 rules against a fresh deploy's 14 and 76, with the missing four
    // matching this list one-for-one and adoptions at 0 instead of 3.
    //
    // That is worse than untidiness. Every later test that resolves a rule ran
    // against a database no install ever has, and the state depended on whether
    // this test had run yet — so a governance test could pass alone and fail in
    // the suite, or the reverse.
    //
    // The import is idempotent (asserted above by calling it twice), and it
    // writes precisely what the deploy writes, so leaving the rows in place
    // restores the database to its deployed state rather than to an empty one.
}

#[tokio::test]
async fn resolve_local_checker_rules_returns_only_checker_backed_rules() {
    // D-CHECKER: resolve_local_checker_rules surfaces ONLY adopted rules with
    // verification='checker' + a checker_ref — a 'review' rule in the same pack
    // must not appear. Uses a general adoption so a random folder resolves it.
    let pg = PgStore::test_store().await;
    let pool = pg.pool();
    sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = 'checker-resolve-test'")
        .execute(pool)
        .await
        .unwrap();
    sqlx_core::query::query(
        "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('general', 'General', 0, false) ON CONFLICT (key) DO NOTHING",
    )
    .execute(pool)
    .await
    .unwrap();
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
             VALUES ($1, $2, 1, 'test')",
    )
    .bind(pack)
    .bind(ns)
    .execute(pool)
    .await
    .unwrap();

    let rules = pg.resolve_local_checker_rules(&uuid::Uuid::new_v4()).await.unwrap();
    let mine: Vec<_> =
        rules.iter().filter(|(s, _)| s == "run the linter" || s == "a manual rule").collect();
    assert_eq!(mine.len(), 1, "only the checker-backed rule resolves, not the review rule");
    assert_eq!(mine[0], &("run the linter".to_string(), "lint".to_string()));

    sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE slug = 'checker-resolve-test'")
        .execute(pool)
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.namespaces WHERE id = $1")
        .bind(ns)
        .execute(pool)
        .await
        .unwrap();
}

/// **A STACK IS A PACK APPLIED AT THE REPOSITORY** (#318 R6, D29/D33). The scan
/// adopts a detected stack's `stack:<slug>` pack at the repository's own
/// namespace, made on first use; the pack's rules then resolve for every folder
/// of that repository at the `repository` rung — the reach the retired
/// `technology` binding had. Adopting twice changes nothing.
///
/// Mutations that must break this: slug the repository namespace by name;
/// adopt at a namespace the folder does not reach; tag the rule with the
/// pack's area instead of the rung it was adopted at.
#[tokio::test]
async fn a_stack_pack_adopted_at_a_repository_governs_its_folders() {
    let pg = PgStore::test_store().await;
    let pool = pg.pool();
    let uniq = uuid::Uuid::new_v4().simple().to_string();
    let suffix = format!("r6-stack-{}", &uniq[..8]);
    let fid = super::tests::create_test_folder(&pg, &suffix).await;
    let rid = pg
        .repository_id_for_folder(&fid)
        .await
        .unwrap()
        .expect("the test folder carries a repository");
    let stack = format!("r6zig{}", &uniq[..8]);

    let pack = pg.adopt_stack_pack(&rid, &stack, "Zig").await.unwrap();
    assert_eq!(pg.adopt_stack_pack(&rid, &stack, "Zig").await.unwrap(), pack, "one pack per stack");
    sqlx_core::query::query(
        "INSERT INTO sensei.rule_pack_rules (pack_id, ordinal, statement, body, enforcement)
         VALUES ($1, 0, 'R6 zig rule', 'R6: no hidden allocations', 'required')",
    )
    .bind(pack)
    .execute(pool)
    .await
    .unwrap();

    let raws = pg.resolve_local_pack_raws(Some(&fid)).await.unwrap();
    let mine: Vec<_> = raws.iter().filter(|r| r.title == "R6 zig rule").collect();
    assert_eq!(mine.len(), 1, "the stack pack's rule reaches the repository's folder once");
    assert_eq!(mine[0].scope, "repository", "at the rung it was adopted at");
    assert_eq!(mine[0].enforcement, "required");

    let repo_slug = format!("test/{suffix}");
    assert!(
        pg.folder_namespace_pairs(&fid)
            .await
            .unwrap()
            .contains(&("repository".to_string(), repo_slug.clone())),
        "the repository's own namespace is slugged by its repo_key and bound to it"
    );
    let ns = pg.namespace_for_folder_scope(&fid, "repository").await.unwrap();
    assert!(ns.is_some(), "the repository rung resolves for the folder");

    sqlx_core::query::query("DELETE FROM sensei.rule_packs WHERE id = $1")
        .bind(pack)
        .execute(pool)
        .await
        .unwrap();
    sqlx_core::query::query(
        "DELETE FROM sensei.namespaces WHERE scope_key = 'repository' AND slug = $1",
    )
    .bind(&repo_slug)
    .execute(pool)
    .await
    .unwrap();
}

/// **THE REPOSITORY RUNG IS MADE ON FIRST USE** (#318 R6, D33). A rule or a
/// stance asked for at `repository` scope lands on the repository's own
/// namespace even before any stack made it — its identity is the repository's,
/// so there is nothing to declare first. A folder with no repository has no
/// repository rung: `None`, never an invented namespace.
#[tokio::test]
async fn the_repository_rung_resolves_on_first_use() {
    let pg = PgStore::test_store().await;
    let uniq = uuid::Uuid::new_v4().simple().to_string();
    let suffix = format!("r6-first-{}", &uniq[..8]);
    let fid = super::tests::create_test_folder(&pg, &suffix).await;

    let first = pg.namespace_for_folder_scope(&fid, "repository").await.unwrap();
    assert!(first.is_some(), "the repository rung resolves without a prior stack");
    assert_eq!(
        pg.namespace_for_folder_scope(&fid, "repository").await.unwrap(),
        first,
        "the same namespace every time"
    );

    let loose =
        super::tests::create_test_folder_unattributed(&pg, &format!("r6-loose-{}", &uniq[..8]))
            .await;
    assert_eq!(
        pg.namespace_for_folder_scope(&loose, "repository").await.unwrap(),
        None,
        "no repository, no repository rung"
    );

    sqlx_core::query::query(
        "DELETE FROM sensei.namespaces WHERE scope_key = 'repository' AND slug = $1",
    )
    .bind(format!("test/{suffix}"))
    .execute(pg.pool())
    .await
    .unwrap();
}
