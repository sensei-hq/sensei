    use super::*;

    /// Both learned-rule tests begin with a GLOBAL
    /// `delete from sensei.playbook_rules where source='learned'` as their clean
    /// slate, then insert and read back their own proposal. Run in parallel they
    /// wipe each other's fixture mid-test: the row is inserted, the sibling's
    /// delete removes it, and the `.find(...).unwrap()` that follows panics.
    /// The table-wide delete IS the intended setup, so it can't be scoped per
    /// test — serialise instead. See `crate::tasks::test_support::TestGate` for
    /// why the gate blocks rather than awaiting.
    static LEARNED_RULES_LOCK: crate::tasks::test_support::TestGate =
        crate::tasks::test_support::TestGate::new();

    #[tokio::test]
    async fn playbook_rules_load_and_run_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };

        let rules = pg.list_playbook_rules().await.unwrap();
        assert!(rules.iter().any(|r| r.playbook == "spec_driven"));

        let playbooks = pg.list_playbooks().await.unwrap();
        assert!(playbooks.iter().any(|p| p["name"] == "spec_driven"));

        let guide = pg.list_intake_guide().await.unwrap();
        assert!(guide.iter().any(|g| g["kind"] == "frame"));

        let (proj, _) = pg.get_or_create_project_by_name("_test:playbook_roundtrip").await.unwrap();
        let run_id = pg.insert_playbook_run(
            None, None, "greenfield", "feature", "high",
            None, "spec_driven", "hi", true,
            Some("manual"), false, proj,
        ).await.unwrap();

        let row: (String, String, String, String, bool) = sqlx_core::query_as::query_as(
            "SELECT lifecycle::text, intent::text, risk::text, playbook, confirmed
               FROM sensei.playbook_run WHERE id = $1"
        ).bind(run_id).fetch_one(&pg.pool).await.unwrap();
        assert_eq!(row, ("greenfield".into(), "feature".into(), "high".into(), "spec_driven".into(), true));

        sqlx_core::query::query("DELETE FROM sensei.playbook_run WHERE id = $1")
            .bind(run_id).execute(&pg.pool).await.unwrap();
    }

    #[tokio::test]
    async fn get_playbook_tone() {
        let Ok(pg) = PgStore::connect_test().await else { return; };

        let pb = pg.get_playbook("debug_flow").await.unwrap().unwrap();
        assert_eq!(pb["name"], "debug_flow");
        assert!(!pb["opening_tone"].as_str().unwrap().is_empty());

        assert!(pg.get_playbook("_test:no_such_playbook").await.unwrap().is_none());
    }

    /// A session's nudge gate flips false → true once a *confirmed*
    /// playbook_run is recorded against it — this is the query `hook_nudge`
    /// (api/handlers/sessions.rs) uses to decide whether to suggest
    /// `/sensei:intake`. Mirrors `create_test_folder` from the sibling
    /// `tests` module inline since that helper isn't visible here.
    #[tokio::test]
    async fn session_confirmed_run_gate() {
        let Ok(pg) = PgStore::connect_test().await else { return; };

        pg.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) \
             VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) \
             ON CONFLICT DO NOTHING"
        ).await.unwrap();
        let suffix = format!("nudge_{}", uuid::Uuid::new_v4());
        let abs_path = format!("/_test/{}", suffix);
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path) \
             VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2) \
             ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        ).bind(&suffix).bind(&abs_path).fetch_one(&pg.pool).await.unwrap();
        let fid = row.0;

        let sid = pg.create_session(&fid, "intake test", None).await.unwrap();
        assert!(!pg.session_has_confirmed_run(&sid).await.unwrap());

        let (proj, _) = pg.get_or_create_project_by_name("_test:nudge_gate").await.unwrap();
        pg.insert_playbook_run(
            Some(sid), None, "stable", "bug", "low",
            None, "debug_flow", "r", true,
            None, false, proj,
        ).await.unwrap();
        assert!(pg.session_has_confirmed_run(&sid).await.unwrap());
        // clean slate — shared test DB; this combo is also asserted exactly by
        // playbook_combo_trust_counts_ftr
        pg.execute_raw(&format!("delete from sensei.playbook_run where session_id = '{sid}'")).await.ok();
    }

    /// The §9 attribution join: a confirmed playbook_run picks up its session's
    /// outcome/ftr, feeds the per-combo FTR aggregate, and is idempotent (a
    /// second attribution pass touches nothing new). Mirrors the
    /// `session_confirmed_run_gate` folder/session setup above.
    #[tokio::test]
    async fn attribution_and_stats_roundtrip() {
        let Ok(pg) = PgStore::connect_test().await else { return; };

        pg.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) \
             VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) \
             ON CONFLICT DO NOTHING"
        ).await.unwrap();
        let suffix = format!("attrib_{}", uuid::Uuid::new_v4());
        let abs_path = format!("/_test/{}", suffix);
        let row: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path) \
             VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2) \
             ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        ).bind(&suffix).bind(&abs_path).fetch_one(&pg.pool).await.unwrap();
        let fid = row.0;

        // a confirmed run linked to a session with a known ftr
        let sid = pg.create_session(&fid, "§9 test", None).await.unwrap();
        pg.execute_raw(&format!(
            "update activity.sessions set outcome='completed', ftr=true where id='{sid}'"
        )).await.unwrap();
        let (proj, _) = pg.get_or_create_project_by_name("_test:attrib").await.unwrap();
        pg.insert_playbook_run(
            Some(sid), None, "stable", "bug", "low",
            None, "debug_flow", "r", true, Some("manual"), false, proj,
        ).await.unwrap();

        let n = pg.attribute_playbook_outcomes().await.unwrap();
        assert!(n >= 1);

        let stats = pg.playbook_combo_stats().await.unwrap();
        assert!(stats.iter().any(|s| s.playbook == "debug_flow" && s.n >= 1));

        // idempotent: second attribution touches 0 new
        assert_eq!(pg.attribute_playbook_outcomes().await.unwrap(), 0);
        // clean slate — shared test DB; this combo is also asserted exactly by
        // playbook_combo_trust_counts_ftr
        pg.execute_raw(&format!("delete from sensei.playbook_run where session_id = '{sid}'")).await.ok();
    }

    #[tokio::test]
    async fn apply_learn_plan_reweights_and_upserts() {
        let _guard = LEARNED_RULES_LOCK.enter();
        let Ok(pg) = PgStore::connect_test().await else { return; };
        pg.execute_raw("delete from sensei.playbook_rules where source='learned'").await.ok(); // clean slate — shared test DB
        let rules = pg.list_playbook_rules().await.unwrap();
        let debug = rules.iter().find(|r| r.playbook == "debug_flow").unwrap();
        use crate::playbook::{LearnPlan, LearnedRule, Lifecycle, Intent, Risk};
        let plan = LearnPlan {
            reweights: vec![(debug.id.unwrap(), debug.base_priority + 5)],
            proposals: vec![LearnedRule { lifecycle: Lifecycle::Stable, intent: Intent::Feature,
                risk: Risk::Low, playbook: "mockup_first".into(), priority: 200, rationale: "t".into() }],
        };
        pg.apply_learn_plan(&plan).await.unwrap();
        let after = pg.list_playbook_rules().await.unwrap();
        assert_eq!(after.iter().find(|r| r.id == debug.id).unwrap().priority, debug.base_priority + 5);
        // proposal is enabled=false → NOT in the resolver-visible list_playbook_rules (which filters WHERE enabled)
        let proposals = pg.list_playbook_rule_proposals().await.unwrap();
        assert!(proposals.iter().any(|p| p["playbook"] == "mockup_first"));
        pg.apply_learn_plan(&plan).await.unwrap();   // idempotent upsert
        assert_eq!(pg.list_playbook_rule_proposals().await.unwrap().iter().filter(|p| p["playbook"]=="mockup_first").count(), 1);
    }

    #[tokio::test]
    async fn accept_flips_proposal_enabled() {
        let _guard = LEARNED_RULES_LOCK.enter();
        let Ok(pg) = PgStore::connect_test().await else { return; };
        pg.execute_raw("delete from sensei.playbook_rules where source='learned'").await.ok(); // clean slate — shared test DB
        use crate::playbook::{LearnPlan, LearnedRule, Lifecycle, Intent, Risk};
        pg.apply_learn_plan(&LearnPlan { reweights: vec![], proposals: vec![LearnedRule {
            lifecycle: Lifecycle::Greenfield, intent: Intent::Ux, risk: Risk::High,
            playbook: "spec_driven".into(), priority: 205, rationale: "t".into() }] }).await.unwrap();
        let props = pg.list_playbook_rule_proposals().await.unwrap();
        let id = props.iter().find(|p| p["playbook"]=="spec_driven").unwrap()["id"].as_str().unwrap().to_string();
        // A real learned proposal flips → returns true AND persists (visible to the resolver list).
        assert!(pg.accept_playbook_rule(&id.parse().unwrap()).await.unwrap(), "accepting a real learned proposal returns true");
        assert!(pg.list_playbook_rules().await.unwrap().iter().any(|r| r.id == Some(id.parse().unwrap())));
        // A nonexistent id flips NOTHING → returns false (never a fabricated success).
        assert!(!pg.accept_playbook_rule(&uuid::Uuid::new_v4()).await.unwrap(),
            "accepting an unknown id returns false, not a fabricated accept");
    }

    #[tokio::test]
    async fn find_duplicates_scoped_surfaces_same_folder_pairs() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let u = uuid::Uuid::new_v4();
        let pid = pg.create_project(&format!("_dupproj_{u}"), None, None).await.unwrap();
        pg.execute_raw("INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000002','/_dup','_dup','watching'::sensei.watch_status) ON CONFLICT DO NOTHING").await.unwrap();
        let fid = uuid::Uuid::new_v4();
        pg.execute_raw(&format!(
            "INSERT INTO sensei.folders(id, root_id, kind, name, path, abs_path, project_id) VALUES('{fid}','00000000-0000-0000-0000-000000000002','git'::sensei.folder_kind,'_dup_{u}','_dup','/_dup/{u}','{pid}')"
        )).await.unwrap();
        // Two near-identical function nodes in the SAME folder (identical 384-dim
        // embedding → similarity 1.0). The old cross-folder-only predicate hid these.
        let emb = "(select '['||string_agg('0.1',',')||']' from generate_series(1,384))::vector";
        for n in ["_dupfn_a", "_dupfn_b"] {
            pg.execute_raw(&format!(
                "INSERT INTO sensei.nodes(folder_id, kind, name, file_path, line_start, line_end, embedding) \
                 VALUES('{fid}','function'::sensei.node_kind,'{n}','/_dup/{u}/x.rs',1,10,{emb})"
            )).await.unwrap();
        }
        let dups = pg.find_duplicates_scoped(&[fid], 0.9, 50).await.unwrap();
        assert!(dups.iter().any(|d| {
            let names = (d["a"]["name"].as_str(), d["b"]["name"].as_str());
            names == (Some("_dupfn_a"), Some("_dupfn_b")) || names == (Some("_dupfn_b"), Some("_dupfn_a"))
        }), "same-folder near-duplicate functions must surface (regression: the cross-folder-only predicate masked all monorepo dupes)");
    }

    #[tokio::test]
    async fn patterns_for_symbol_matches_by_file_and_is_honest_empty() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let u = uuid::Uuid::new_v4();
        let pid = pg.create_project(&format!("_pfsproj_{u}"), None, None).await.unwrap();
        pg.execute_raw("INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000003','/_pfs','_pfs','watching'::sensei.watch_status) ON CONFLICT DO NOTHING").await.unwrap();
        let fid = uuid::Uuid::new_v4();
        pg.execute_raw(&format!("INSERT INTO sensei.folders(id, root_id, kind, name, path, abs_path, project_id) VALUES('{fid}','00000000-0000-0000-0000-000000000003','git'::sensei.folder_kind,'_pfs_{u}','_pfs','/_pfs/{u}','{pid}')")).await.unwrap();
        // A node 'my_handler' at a repo-RELATIVE path; a project pattern whose instance is its ABSOLUTE form.
        pg.execute_raw(&format!("INSERT INTO sensei.nodes(folder_id, kind, name, file_path, line_start, line_end) VALUES('{fid}','function'::sensei.node_kind,'my_handler','src/routes/x.rs',1,10)")).await.unwrap();
        pg.execute_raw(&format!("INSERT INTO inference.detected_patterns(project_id, name, family, instance_count, instances) VALUES('{pid}','route-handler','route',1,'[{{\"file\":\"/_pfs/{u}/src/routes/x.rs\",\"line\":1}}]'::jsonb)")).await.unwrap();
        // The symbol's file IS in the pattern's instances (abs↔rel reconciled) → match.
        let hit = pg.patterns_for_symbol(&pid, &[fid], "my_handler").await.unwrap();
        assert!(hit.iter().any(|p| p["name"] == "route-handler"),
            "symbol's file matches the pattern instance (was always-null against a nonexistent members field)");
        // A symbol in no pattern → honest empty, never a fabricated null.
        let miss = pg.patterns_for_symbol(&pid, &[fid], "not_a_symbol").await.unwrap();
        assert!(miss.is_empty(), "no file membership → honest empty");
    }

    #[tokio::test]
    async fn playbook_combo_trust_is_project_scoped() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let (proj_a, _) = pg.get_or_create_project_by_name("_test:trust_a").await.unwrap();
        let (proj_b, _) = pg.get_or_create_project_by_name("_test:trust_b").await.unwrap();
        pg.execute_raw("delete from sensei.playbook_run where feature = 'trust-test'").await.ok();
        // project A: two confirmed+attributed runs for (stable,bug,low, debug_flow): one ftr, one not → n=2, ftr=0.5
        for ftr in ["true", "false"] {
            pg.execute_raw(&format!(
                "insert into sensei.playbook_run (feature, lifecycle, intent, risk, playbook, rationale, confirmed, outcome_ftr, project_id) \
                 values ('trust-test','stable','bug','low','debug_flow','t', true, {ftr}, '{proj_a}')")).await.unwrap();
        }
        // project B: one confirmed run, ftr true — must NOT bleed into A's trust
        pg.execute_raw(&format!(
            "insert into sensei.playbook_run (feature, lifecycle, intent, risk, playbook, rationale, confirmed, outcome_ftr, project_id) \
             values ('trust-test','stable','bug','low','debug_flow','t', true, true, '{proj_b}')")).await.unwrap();

        // scoped to A: only A's 2 runs → n=2, ftr=0.5 (B's run excluded — trust is per-project, never global)
        let (na, fa) = pg.playbook_combo_trust("stable","bug","low","debug_flow", &proj_a).await.unwrap();
        assert_eq!(na, 2, "trust must count only the in-scope project's runs");
        assert!((fa - 0.5).abs() < 1e-9);
        // scoped to B: only B's run → n=1, ftr=1.0
        let (nb, fb) = pg.playbook_combo_trust("stable","bug","low","debug_flow", &proj_b).await.unwrap();
        assert_eq!(nb, 1);
        assert!((fb - 1.0).abs() < 1e-9);
        // empty combo in A → (0, 0.0)
        let (n0, f0) = pg.playbook_combo_trust("greenfield","ux","high","vibe", &proj_a).await.unwrap();
        assert_eq!(n0, 0); assert_eq!(f0, 0.0);
        pg.execute_raw("delete from sensei.playbook_run where feature = 'trust-test'").await.ok();
    }

    #[tokio::test]
    async fn model_stats_groups_by_classified_by() {
        let Ok(pg) = PgStore::connect_test().await else { return; };
        let rows = pg.playbook_model_stats().await.unwrap();
        // shape check: each row has classified_by + n + ftr_rate keys (may be empty on a fresh DB)
        if let Some(r) = rows.first() { assert!(r.get("classified_by").is_some() && r.get("ftr_rate").is_some()); }
    }
