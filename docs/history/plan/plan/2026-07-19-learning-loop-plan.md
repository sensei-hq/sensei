# §9 Learning Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the front-door loop — attribute each confirmed `playbook_run`'s FTR outcome back from its session, then adapt `playbook_rules` (bounded auto-reweight of existing rules + proposed new learned rules) so the recommender compounds.

**Architecture:** An idempotent analyzer global-pass stage attributes outcomes (`playbook_run.outcome`/`outcome_ftr` from `activity.sessions`), aggregates per-(axes×playbook) FTR stats, runs a **pure `learn()` policy** (reweight + propose), and applies it (UPDATE priorities in place off an immutable `base_priority`; UPSERT `source='learned', enabled=false` proposals). An accept path (endpoints + MCP tools) flips a proposal `enabled=true`. A model-stats read exposes FTR by `classified_by`.

**Tech Stack:** PostgreSQL + `dbd`; Rust (`crates/senseid` bin, `crates/mcp`); `sqlx_core::query_as`; the analyzer in `crates/senseid/src/tasks/`. Tests: `cargo test -p senseid -- --test-threads=1` (a pre-existing `prune_activity_*` test flakes under parallel — single-threaded avoids it). DB tests use `sensei_test` (`postgresql://localhost:5432/sensei_test`, already seeded 6 playbooks/6 rules/4 guide). Commit per task to `develop` (approach A).

**Design:** `docs/plan/2026-07-19-learning-loop-design.md`. **Prereq merged:** the front-door feature (Plans 1+2), HEAD at/after `f463e30c`.

**GIT HYGIENE:** the pre-commit hook stages broadly — before each commit run `git status` and `git add` only the task's files; never sweep.

---

## File Structure

**Modify (DDL):**
- `database/ddl/table/sensei/playbook_run.ddl` — add `outcome_ftr boolean`.
- `database/ddl/table/sensei/playbook_rules.ddl` — add `base_priority integer` + the learned partial-unique index.
- `database/ddl/procedure/staging/import_playbook_rules.ddl` — set `base_priority := priority` on insert.

**Modify (Rust):**
- `crates/senseid/src/playbook.rs` — add `base_priority` to `Rule`; add the pure learning policy (`ComboPlaybookStat`, `LearnedRule`, `LearnPlan`, `learn()`) + unit tests.
- `crates/senseid/src/db/pg_store.rs` — `attribute_playbook_outcomes`, `playbook_combo_stats`, extend `list_playbook_rules` to read `base_priority`, `apply_learn_plan`, `list_playbook_rule_proposals`, `accept_playbook_rule`, `playbook_model_stats`.
- `crates/senseid/src/api/handlers/playbook.rs` — `list_rule_proposals`, `accept_rule`, `model_stats` handlers.
- `crates/senseid/src/api/routes.rs` — 3 routes.
- `crates/mcp/src/lib.rs` — `list_playbook_rule_proposals` + `accept_playbook_rule` tools + `EXPECTED_TOOLS`.
- `crates/senseid/src/tasks/analyzer_scheduler.rs` + `crates/senseid/src/tasks/handlers/` — the global-pass stage that orchestrates attribute→stats→learn→apply.

---

### Task 1: DDL — attribution + reweight columns

**Files:**
- Modify: `database/ddl/table/sensei/playbook_run.ddl`, `database/ddl/table/sensei/playbook_rules.ddl`, `database/ddl/procedure/staging/import_playbook_rules.ddl`

- [ ] **Step 1: Edit the `.ddl` files (full DDL, no ALTER in the file).**

In `playbook_run.ddl`, add before `created_at`:
```sql
, outcome_ftr boolean
```
In `playbook_rules.ddl`, add `base_priority` after `priority` and add the index after the existing `playbook_rules_match_idx`:
```sql
, base_priority   integer
```
```sql
create unique index if not exists playbook_rules_learned_uq
    on playbook_rules(match_lifecycle, match_intent, match_risk, playbook)
    where source = 'learned';
```
In `import_playbook_rules.ddl`, add `base_priority` to the insert column list + `stg.priority` to values (base defaults to the seed priority):
```sql
  insert into sensei.playbook_rules
     (name, match_lifecycle, match_intent, match_risk, playbook, rationale, priority, base_priority, enabled, source)
  select name,
         nullif(match_lifecycle,'')::sensei.chunk_lifecycle,
         nullif(match_intent,'')::sensei.chunk_intent,
         nullif(match_risk,'')::sensei.chunk_risk,
         playbook, rationale, priority, priority, coalesce(enabled,true), coalesce(source,'builtin')
    from staging.playbook_rules stg
  where not exists (select 1 from sensei.playbook_rules r where r.name = stg.name and r.source='builtin');
```

- [ ] **Step 2: Apply additively to `sensei_test` + verify.**

Run:
```bash
T="postgresql://localhost:5432/sensei_test"
psql "$T" -c "alter table sensei.playbook_run add column if not exists outcome_ftr boolean;"
psql "$T" -c "alter table sensei.playbook_rules add column if not exists base_priority integer;"
psql "$T" -c "update sensei.playbook_rules set base_priority = priority where base_priority is null;"
psql "$T" -f database/ddl/procedure/staging/import_playbook_rules.ddl
psql "$T" -c "create unique index if not exists playbook_rules_learned_uq on sensei.playbook_rules(match_lifecycle, match_intent, match_risk, playbook) where source='learned';"
psql "$T" -tAc "select count(*) from sensei.playbook_rules where base_priority is not null;"   # Expected: 6
psql "$T" -c "\d sensei.playbook_run" | grep outcome_ftr
```
Expected: 6 rules with `base_priority`; `outcome_ftr | boolean` present.

- [ ] **Step 3: Commit**
```bash
git add database/ddl/table/sensei/playbook_run.ddl database/ddl/table/sensei/playbook_rules.ddl database/ddl/procedure/staging/import_playbook_rules.ddl
git commit -m "feat(db): §9 columns — playbook_run.outcome_ftr + playbook_rules.base_priority + learned unique index"
```

---

### Task 2: Pure learning policy `learn()`

**Files:**
- Modify: `crates/senseid/src/playbook.rs`

- [ ] **Step 1: Add `base_priority` to `Rule` + write the failing tests.**

Add the field to `Rule` (after `priority`): `pub base_priority: i32,`. Update the 3 existing resolver tests' `Rule { … }` literals to include `base_priority: <same as priority>` (they won't compile otherwise). Then append to the `#[cfg(test)] mod tests`:
```rust
    fn stat(l: Lifecycle, i: Intent, r: Risk, pb: &str, n: i64, ftr: f64) -> ComboPlaybookStat {
        ComboPlaybookStat { lifecycle: l, intent: i, risk: r, playbook: pb.into(), n, ftr_rate: ftr }
    }
    fn rule(id: u128, l: Option<Lifecycle>, i: Option<Intent>, r: Option<Risk>, pb: &str, prio: i32) -> Rule {
        Rule { id: Some(uuid::Uuid::from_u128(id)), name: pb.into(), match_lifecycle: l, match_intent: i,
               match_risk: r, playbook: pb.into(), rationale: "r".into(), priority: prio, base_priority: prio }
    }

    #[test]
    fn reweight_bumps_priority_up_for_strong_ftr() {
        let rules = vec![rule(1, Some(Lifecycle::Stable), Some(Intent::Bug), None, "debug_flow", 60)];
        let stats = vec![stat(Lifecycle::Stable, Intent::Bug, Risk::Low, "debug_flow", 10, 1.0)];
        let plan = learn(&stats, &rules);
        let (_, np) = plan.reweights.iter().find(|(id,_)| *id == rules[0].id.unwrap()).unwrap();
        assert!(*np > 60, "high FTR should raise priority (got {np})");
        assert!(*np <= 60 + 20, "bounded by REWEIGHT_BOUND");
    }

    #[test]
    fn reweight_ignored_below_min_sample() {
        let rules = vec![rule(1, Some(Lifecycle::Stable), Some(Intent::Bug), None, "debug_flow", 60)];
        let stats = vec![stat(Lifecycle::Stable, Intent::Bug, Risk::Low, "debug_flow", 3, 1.0)];
        assert!(learn(&stats, &rules).reweights.is_empty(), "n<5 → no reweight");
    }

    #[test]
    fn reweight_is_idempotent() {
        let mut rules = vec![rule(1, Some(Lifecycle::Stable), Some(Intent::Bug), None, "debug_flow", 60)];
        let stats = vec![stat(Lifecycle::Stable, Intent::Bug, Risk::Low, "debug_flow", 10, 1.0)];
        let np = learn(&stats, &rules).reweights[0].1;
        rules[0].priority = np;                 // apply once (base_priority stays 60)
        assert_eq!(learn(&stats, &rules).reweights.iter().find(|(_,p)| *p != np), None,
                   "same stats + same base → same target priority");
    }

    #[test]
    fn proposes_better_playbook_over_recommended() {
        // recommended for (stable,feature,low) = gsd (seed). But mockup_first scores far higher here.
        let rules = vec![rule(1, None, Some(Intent::Feature), Some(Risk::Low), "gsd", 40)];
        let stats = vec![
            stat(Lifecycle::Stable, Intent::Feature, Risk::Low, "gsd", 8, 0.4),
            stat(Lifecycle::Stable, Intent::Feature, Risk::Low, "mockup_first", 8, 0.9),
        ];
        let plan = learn(&stats, &rules);
        let p = plan.proposals.iter().find(|p| p.playbook == "mockup_first").expect("propose the winner");
        assert_eq!((p.lifecycle, p.intent, p.risk), (Lifecycle::Stable, Intent::Feature, Risk::Low));
        assert!(p.priority > 40, "must out-prioritize the recommended rule");
    }

    #[test]
    fn no_proposal_when_recommended_is_best_or_delta_small() {
        let rules = vec![rule(1, None, Some(Intent::Feature), Some(Risk::Low), "gsd", 40)];
        let stats = vec![
            stat(Lifecycle::Stable, Intent::Feature, Risk::Low, "gsd", 8, 0.8),
            stat(Lifecycle::Stable, Intent::Feature, Risk::Low, "vibe", 8, 0.85), // delta 0.05 < 0.2
        ];
        assert!(learn(&stats, &rules).proposals.is_empty());
    }
```

- [ ] **Step 2: Run + confirm fail**

Run: `cargo test -p senseid --bin senseid playbook 2>&1 | tail -6`
Expected: FAIL — `ComboPlaybookStat`/`learn`/`base_priority` not found.

- [ ] **Step 3: Implement the policy** (append to `crates/senseid/src/playbook.rs`, above the test module)
```rust
// ── §9 learning policy (pure) ──────────────────────────────────────────────
const MIN_SAMPLE: i64 = 5;
const FTR_DELTA: f64 = 0.2;
const REWEIGHT_K: f64 = 40.0;
const REWEIGHT_BOUND: i32 = 20;
const REWEIGHT_TARGET_FTR: f64 = 0.5; // neutral FTR midpoint the reweight measures against

#[derive(Clone, Debug)]
pub struct ComboPlaybookStat {
    pub lifecycle: Lifecycle, pub intent: Intent, pub risk: Risk,
    pub playbook: String, pub n: i64, pub ftr_rate: f64,
}

#[derive(Clone, Debug)]
pub struct LearnedRule {
    pub lifecycle: Lifecycle, pub intent: Intent, pub risk: Risk,
    pub playbook: String, pub priority: i32, pub rationale: String,
}

#[derive(Clone, Debug, Default)]
pub struct LearnPlan {
    pub reweights: Vec<(uuid::Uuid, i32)>,   // (rule_id, new_priority)
    pub proposals: Vec<LearnedRule>,
}

fn stat_matches_rule(s: &ComboPlaybookStat, r: &Rule) -> bool {
    r.match_lifecycle.map_or(true, |m| m == s.lifecycle)
        && r.match_intent.map_or(true, |m| m == s.intent)
        && r.match_risk.map_or(true, |m| m == s.risk)
}

/// Pure: current per-(axes×playbook) FTR stats + the live rule set → a plan of
/// bounded priority reweights (existing rules) + proposed new learned rules.
pub fn learn(stats: &[ComboPlaybookStat], rules: &[Rule]) -> LearnPlan {
    let mut plan = LearnPlan::default();

    // Reweight: each rule scored on its playbook's FTR across the combos it matches,
    // measured against a fixed neutral target (REWEIGHT_TARGET_FTR) — robust and
    // degeneracy-free (no dependence on the mix of other data).
    for r in rules {
        let matching: Vec<&ComboPlaybookStat> =
            stats.iter().filter(|s| s.playbook == r.playbook && stat_matches_rule(s, r)).collect();
        let n: i64 = matching.iter().map(|s| s.n).sum();
        if n < MIN_SAMPLE {
            continue;
        }
        let ftr = matching.iter().map(|s| s.ftr_rate * s.n as f64).sum::<f64>() / n as f64;
        let adj = ((REWEIGHT_K * (ftr - REWEIGHT_TARGET_FTR)).round() as i32)
            .clamp(-REWEIGHT_BOUND, REWEIGHT_BOUND);
        let new_priority = r.base_priority + adj;
        if let Some(id) = r.id {
            if new_priority != r.priority {
                plan.reweights.push((id, new_priority));
            }
        }
    }

    // Propose: for each exact combo, if the best-performing playbook beats the
    // currently-recommended one by >= FTR_DELTA (with enough samples), propose it.
    let mut combos: Vec<(Lifecycle, Intent, Risk)> =
        stats.iter().map(|s| (s.lifecycle, s.intent, s.risk)).collect();
    combos.sort_by_key(|(l, i, r)| (l.as_str(), i.as_str(), r.as_str()));
    combos.dedup();
    for (l, i, rk) in combos {
        let axes = Axes { lifecycle: l, intent: i, risk: rk };
        let here: Vec<&ComboPlaybookStat> = stats.iter()
            .filter(|s| s.lifecycle == l && s.intent == i && s.risk == rk && s.n >= MIN_SAMPLE)
            .collect();
        let Some(best) = here.iter().max_by(|a, b| a.ftr_rate.total_cmp(&b.ftr_rate)) else { continue };
        let rec = recommend(&axes, rules);
        let rec_ftr = here.iter().find(|s| s.playbook == rec.playbook).map_or(0.0, |s| s.ftr_rate);
        if best.playbook != rec.playbook && best.ftr_rate - rec_ftr >= FTR_DELTA {
            let top = rules.iter().filter(|r| stat_matches_rule(here[0], r))
                .map(|r| r.priority).max().unwrap_or(0);
            plan.proposals.push(LearnedRule {
                lifecycle: l, intent: i, risk: rk, playbook: best.playbook.clone(),
                priority: top + 1,
                rationale: format!(
                    "learned: {} out-performed {} here (FTR {:.2} vs {:.2}, n={})",
                    best.playbook, rec.playbook, best.ftr_rate, rec_ftr, best.n),
            });
        }
    }
    plan
}
```
NB: `stat_matches_rule` mirrors the resolver's `matches`; `here[0]` is any stat for the combo (all share the axes), used only to test which rules cover the combo.

- [ ] **Step 4: Run tests + clippy**

Run: `cargo test -p senseid --bin senseid playbook 2>&1 | tail -6` (Expected: all pass — 3 resolver + 5 new)
Run: `cargo clippy -p senseid 2>&1 | tail -3` (Expected: clean — fix any lint you introduce)

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/playbook.rs
git commit -m "feat(senseid): §9 pure learn() policy (bounded reweight + propose learned rules)"
```

---

### Task 3: Attribution + stats + rule-load in pg_store

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1: Write the failing DB test** (mirror the existing `playbook_tests` module + `PgStore::connect_test()`)
```rust
#[tokio::test]
async fn attribution_and_stats_roundtrip() {
    let Ok(pg) = PgStore::connect_test().await else { return; };
    // a confirmed run linked to a session with a known ftr
    let fid = /* create a folder + session as the sibling tests do */;
    let sid = pg.create_session(&fid, "§9 test", None).await.unwrap();
    pg.execute_raw(&format!("update activity.sessions set outcome='completed', ftr=true where id='{sid}'")).await.unwrap();
    pg.insert_playbook_run(Some(sid), None, "stable","bug","low", None, "debug_flow","r", true, Some("manual"), false).await.unwrap();
    let n = pg.attribute_playbook_outcomes().await.unwrap();
    assert!(n >= 1);
    let stats = pg.playbook_combo_stats().await.unwrap();
    assert!(stats.iter().any(|s| s.playbook == "debug_flow" && s.n >= 1));
    // idempotent: second attribution touches 0 new
    assert_eq!(pg.attribute_playbook_outcomes().await.unwrap(), 0);
}
```
(Use the exact folder/session setup the sibling `session_confirmed_run_gate` test already uses.)

- [ ] **Step 2: Run + confirm fail** — `cargo test -p senseid attribution_and_stats_roundtrip 2>&1 | tail -5` → FAIL (methods missing).

- [ ] **Step 3: Implement.** Add `base_priority` to `list_playbook_rules`'s SELECT + `Rule` mapping (`COALESCE(base_priority, priority)`), and add:
```rust
/// Snapshot the session's outcome onto confirmed, not-yet-attributed runs. Returns rows updated.
pub async fn attribute_playbook_outcomes(&self) -> Result<u64, String> {
    let res = sqlx_core::query::query(
        "UPDATE sensei.playbook_run pr
            SET outcome = s.outcome::text, outcome_ftr = s.ftr
           FROM activity.sessions s
          WHERE pr.session_id = s.id AND pr.confirmed
            AND pr.outcome IS NULL AND s.outcome IS NOT NULL"
    ).execute(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(res.rows_affected())
}

pub async fn playbook_combo_stats(&self) -> Result<Vec<crate::playbook::ComboPlaybookStat>, String> {
    use crate::playbook::{ComboPlaybookStat, Lifecycle, Intent, Risk};
    let rows: Vec<(String, String, String, String, i64, f64)> = sqlx_core::query_as::query_as(
        "SELECT lifecycle::text, intent::text, risk::text, playbook,
                count(*)::int8, avg(outcome_ftr::int)::float8
           FROM sensei.playbook_run
          WHERE confirmed AND outcome_ftr IS NOT NULL
          GROUP BY lifecycle, intent, risk, playbook"
    ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().filter_map(|(l,i,r,pb,n,ftr)| Some(ComboPlaybookStat {
        lifecycle: Lifecycle::parse(&l)?, intent: Intent::parse(&i)?, risk: Risk::parse(&r)?,
        playbook: pb, n, ftr_rate: ftr,
    })).collect())
}
```

- [ ] **Step 4: Run tests + clippy** — `cargo test -p senseid attribution_and_stats_roundtrip 2>&1 | tail -5` (PASS/skip); `cargo clippy -p senseid 2>&1 | tail -3` (clean).

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(senseid): §9 pg_store — attribute_playbook_outcomes + playbook_combo_stats + base_priority load"
```

---

### Task 4: Apply the plan + wire the analyzer stage

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`, `crates/senseid/src/tasks/handlers/analyze.rs` (or a new `handlers/learn_playbooks.rs`), `crates/senseid/src/tasks/analyzer_scheduler.rs`

- [ ] **Step 1: Write the failing DB test** for `apply_learn_plan`
```rust
#[tokio::test]
async fn apply_learn_plan_reweights_and_upserts() {
    let Ok(pg) = PgStore::connect_test().await else { return; };
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
```

- [ ] **Step 2: Run + confirm fail** — `cargo test -p senseid apply_learn_plan_reweights_and_upserts 2>&1 | tail -5`.

- [ ] **Step 3: Implement `apply_learn_plan`** (pg_store) — UPDATE priorities; UPSERT proposals via the learned unique index
```rust
pub async fn apply_learn_plan(&self, plan: &crate::playbook::LearnPlan) -> Result<(), String> {
    for (id, new_priority) in &plan.reweights {
        sqlx_core::query::query("UPDATE sensei.playbook_rules SET priority = $2 WHERE id = $1")
            .bind(id).bind(new_priority).execute(&self.pool).await.map_err(|e| e.to_string())?;
    }
    for p in &plan.proposals {
        sqlx_core::query::query(
            "INSERT INTO sensei.playbook_rules
               (name, match_lifecycle, match_intent, match_risk, playbook, rationale,
                priority, base_priority, enabled, source)
             VALUES ($1, $2::sensei.chunk_lifecycle, $3::sensei.chunk_intent, $4::sensei.chunk_risk,
                     $5, $6, $7, $7, false, 'learned')
             ON CONFLICT (match_lifecycle, match_intent, match_risk, playbook)
               WHERE source='learned'
             DO UPDATE SET rationale = excluded.rationale, priority = excluded.priority, base_priority = excluded.priority"
        )
        .bind(format!("learned: {}", p.playbook))
        .bind(p.lifecycle.as_str()).bind(p.intent.as_str()).bind(p.risk.as_str())
        .bind(&p.playbook).bind(&p.rationale).bind(p.priority)
        .execute(&self.pool).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
```
(Also add `list_playbook_rule_proposals` here or in Task 5 — the test needs it; if you implement it now, mirror `list_playbooks`: `SELECT id,name,match_*::text,playbook,rationale,priority,created_at FROM sensei.playbook_rules WHERE source='learned' AND NOT enabled ORDER BY created_at DESC` → `Vec<serde_json::Value>`.)

- [ ] **Step 4: Wire the analyzer stage.** Add a global-pass task variant (mirror `AggregateCorrections` in `analyzer_scheduler.rs`'s global-pass set) whose handler runs:
```rust
// handler: learn_playbooks
let _ = pg.attribute_playbook_outcomes().await;
let stats = pg.playbook_combo_stats().await.unwrap_or_default();
let rules = pg.list_playbook_rules().await.unwrap_or_default();
let plan = crate::playbook::learn(&stats, &rules);
if let Err(e) = pg.apply_learn_plan(&plan).await {
    tracing::warn!(error=%e, "learn_playbooks: apply_learn_plan failed");
} else {
    tracing::info!(reweights=plan.reweights.len(), proposals=plan.proposals.len(), "learn_playbooks: applied");
}
```
Register it in the global-pass set enqueued once per due tick (alongside `AggregateCorrections` etc.), NOT per-project. Follow the exact enum/dispatch shape those tasks use.

- [ ] **Step 5: Run tests + build + clippy** — `cargo test -p senseid apply_learn_plan_reweights_and_upserts 2>&1 | tail -5`; `cargo build -p senseid 2>&1 | tail -3`; `cargo clippy -p senseid --all-targets 2>&1 | tail -3` (clean).

- [ ] **Step 6: Commit**
```bash
git add crates/senseid/src/db/pg_store.rs crates/senseid/src/tasks/
git commit -m "feat(senseid): §9 apply_learn_plan + analyzer global-pass stage (attribute→stats→learn→apply)"
```

---

### Task 5: Accept path — list + accept proposals (endpoints + MCP tools)

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs` (if `list_playbook_rule_proposals` not done in T4; add `accept_playbook_rule`), `crates/senseid/src/api/handlers/playbook.rs`, `crates/senseid/src/api/routes.rs`, `crates/mcp/src/lib.rs`

- [ ] **Step 1: Write the failing DB test**
```rust
#[tokio::test]
async fn accept_flips_proposal_enabled() {
    let Ok(pg) = PgStore::connect_test().await else { return; };
    use crate::playbook::{LearnPlan, LearnedRule, Lifecycle, Intent, Risk};
    pg.apply_learn_plan(&LearnPlan { reweights: vec![], proposals: vec![LearnedRule {
        lifecycle: Lifecycle::Greenfield, intent: Intent::Ux, risk: Risk::High,
        playbook: "spec_driven".into(), priority: 205, rationale: "t".into() }] }).await.unwrap();
    let props = pg.list_playbook_rule_proposals().await.unwrap();
    let id = props.iter().find(|p| p["playbook"]=="spec_driven").unwrap()["id"].as_str().unwrap().to_string();
    pg.accept_playbook_rule(&id.parse().unwrap()).await.unwrap();
    // now enabled → visible to the resolver-facing list
    assert!(pg.list_playbook_rules().await.unwrap().iter().any(|r| r.id == Some(id.parse().unwrap())));
}
```

- [ ] **Step 2: Run + confirm fail.**

- [ ] **Step 3: Implement** `accept_playbook_rule` (pg_store): `UPDATE sensei.playbook_rules SET enabled=true WHERE id=$1 AND source='learned'`. Then handlers in `api/handlers/playbook.rs`:
```rust
pub(crate) async fn list_rule_proposals(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "proposals": state.pg.list_playbook_rule_proposals().await.unwrap_or_default() }))
}
pub(crate) async fn accept_rule(State(state): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<serde_json::Value> {
    let uid = match id.parse() { Ok(u) => u, Err(_) => return Json(serde_json::json!({"error":"invalid id"})) };
    match state.pg.accept_playbook_rule(&uid).await {
        Ok(()) => Json(serde_json::json!({"accepted": id})),
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}
```
Routes: `.route("/api/playbook/rule-proposals", get(playbook::list_rule_proposals))` + `.route("/api/playbook/rule/{id}/accept", post(playbook::accept_rule))`. MCP tools (mirror `get_intake_guide`): `list_playbook_rule_proposals` (no args → `DaemonRequest::get("/api/playbook/rule-proposals")`) + `accept_playbook_rule` (required `id` → `DaemonRequest::post_json(format!("/api/playbook/rule/{id}/accept"), json!({}))`). Add both to `EXPECTED_TOOLS`.

- [ ] **Step 4: Run tests + build + clippy** — DB test PASS/skip; `cargo test -p sensei-mcp 2>&1 | tail -5` (catalog test includes the 2 new tools); `cargo clippy -p senseid -p sensei-mcp --all-targets` clean.

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/db/pg_store.rs crates/senseid/src/api/handlers/playbook.rs crates/senseid/src/api/routes.rs crates/mcp/src/lib.rs
git commit -m "feat(senseid,mcp): §9 accept path — list_playbook_rule_proposals + accept_playbook_rule"
```

---

### Task 6: Model-stats read (local-model metric)

**Files:**
- Modify: `crates/senseid/src/db/pg_store.rs`, `crates/senseid/src/api/handlers/playbook.rs`, `crates/senseid/src/api/routes.rs`

- [ ] **Step 1: Write the failing DB test**
```rust
#[tokio::test]
async fn model_stats_groups_by_classified_by() {
    let Ok(pg) = PgStore::connect_test().await else { return; };
    let rows = pg.playbook_model_stats().await.unwrap();
    // shape check: each row has classified_by + n + ftr_rate keys (may be empty on a fresh DB)
    if let Some(r) = rows.first() { assert!(r.get("classified_by").is_some() && r.get("ftr_rate").is_some()); }
}
```

- [ ] **Step 2: Run + confirm fail.**

- [ ] **Step 3: Implement** `playbook_model_stats` (pg_store):
```rust
pub async fn playbook_model_stats(&self) -> Result<Vec<serde_json::Value>, String> {
    let rows: Vec<(Option<String>, Option<bool>, i64, f64)> = sqlx_core::query_as::query_as(
        "SELECT classified_by, model_fallback, count(*)::int8, avg(outcome_ftr::int)::float8
           FROM sensei.playbook_run WHERE confirmed AND outcome_ftr IS NOT NULL
          GROUP BY classified_by, model_fallback ORDER BY count(*) DESC"
    ).fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|(cb,mf,n,ftr)| serde_json::json!({
        "classified_by": cb, "model_fallback": mf, "n": n, "ftr_rate": ftr
    })).collect())
}
```
Handler `model_stats` (mirror `list_rule_proposals`) → `GET /api/playbook/model-stats`. (No MCP tool needed — this is a dashboard/inspection read; skip unless trivially added.)

- [ ] **Step 4: Run tests + build + clippy** — clean.

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/db/pg_store.rs crates/senseid/src/api/handlers/playbook.rs crates/senseid/src/api/routes.rs
git commit -m "feat(senseid): §9 model-stats read — FTR by classified_by (local-model usefulness)"
```

---

## Final verification (whole plan)

- [ ] `cargo test -p senseid -- --test-threads=1 2>&1 | tail -8` — green (resolver + learn policy + attribution/apply/accept/model-stats DB tests).
- [ ] `cargo test -p sensei-mcp 2>&1 | tail -5` — catalog includes `list_playbook_rule_proposals` + `accept_playbook_rule`.
- [ ] `cargo clippy -p senseid -p sensei-mcp --all-targets 2>&1 | tail -5` — clean.
- [ ] DDL applies on a fresh `sensei_test` (columns + learned index); `import_playbook_rules` sets `base_priority`.
- [ ] Manual: seed a couple confirmed runs + enriched sessions → run the learn handler → verify a reweight and/or a proposal appears; accept a proposal → resolver returns it.
- [ ] Whole-plan final review (subagent) against this plan + the design.

## Self-review notes (author)

- **Spec coverage:** DDL (outcome_ftr/base_priority/learned-index) ✓ T1; pure learn (reweight+propose) ✓ T2; attribution+stats ✓ T3; apply+analyzer wiring ✓ T4; accept path ✓ T5; model-stats ✓ T6.
- **Type consistency:** `Rule.base_priority: i32` added in T2 + loaded in T3; `ComboPlaybookStat`/`LearnedRule`/`LearnPlan` defined in T2, consumed by T3/T4; `apply_learn_plan` signature matches learn()'s output; axis `as_str`/`parse` labels identical to the DDL enums.
- **Proposed = `enabled=false`** throughout; resolver's `WHERE enabled` keeps proposals invisible until `accept` flips it.
- **Open (pin during impl):** the exact analyzer global-pass task enum variant + dispatch (mirror `AggregateCorrections`); the folder/session setup in DB tests (copy the sibling `session_confirmed_run_gate` test's helper).
