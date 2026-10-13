# Auto-select-on-trust Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Low-risk chunks auto-confirm the recommended playbook when §9's FTR history says it's reliable — skip the human confirm, announce, stay reversible.

**Architecture:** A pure `is_trusted()` gate + a focused `playbook_combo_trust` query feed an `auto_select` flag on the `recommend_playbook` response; `/sensei:intake` honors it (auto-confirm + announce). No new schema, no new MCP tool.

**Tech Stack:** Rust (`crates/senseid`), `sqlx_core::query_as`; the marketplace command. Tests: `cargo test -p senseid -- --test-threads=1`; `sensei_test` DB. Commit per task to `develop` (approach A). **GIT HYGIENE:** pre-commit hook stages broadly — `git status` + explicit `git add`, no sweep.

**Design:** `docs/plan/2026-07-19-auto-select-on-trust-design.md`. **Prereq merged:** §9 (HEAD ≥ `e6546cda`).

---

### Task 1: Pure `is_trusted` gate

**Files:** Modify `crates/senseid/src/playbook.rs`

- [ ] **Step 1: Write the failing tests** (append to `#[cfg(test)] mod tests`)
```rust
    #[test]
    fn trusted_only_for_proven_low_risk() {
        assert!(is_trusted(Risk::Low, 10, 0.8));   // boundary: n==MIN, ftr==TARGET
        assert!(is_trusted(Risk::Low, 40, 0.95));
        assert!(!is_trusted(Risk::High, 40, 0.95)); // high-risk never auto-selects
        assert!(!is_trusted(Risk::Low, 9, 0.95));   // too few samples
        assert!(!is_trusted(Risk::Low, 40, 0.79));  // FTR below target
    }
```

- [ ] **Step 2: Run + confirm fail** — `cargo test -p senseid --bin senseid playbook 2>&1 | tail -5` → FAIL (`is_trusted` not found).

- [ ] **Step 3: Implement** (add near the §9 consts + `learn()`)
```rust
const TRUST_MIN_SAMPLE: i64 = 10;
const TRUST_FTR: f64 = 0.8;

/// Auto-select gate: a low-risk chunk whose chosen playbook has enough proven FTR history.
/// Stricter than §9's learn thresholds — skipping a human confirm demands more evidence.
pub fn is_trusted(risk: Risk, n: i64, ftr: f64) -> bool {
    risk == Risk::Low && n >= TRUST_MIN_SAMPLE && ftr >= TRUST_FTR
}
```

- [ ] **Step 4: Run tests + clippy** — `cargo test -p senseid --bin senseid playbook 2>&1 | tail -5` (pass); `cargo clippy -p senseid 2>&1 | tail -3` (clean).

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/playbook.rs
git commit -m "feat(senseid): is_trusted() auto-select gate (low-risk + proven FTR history)"
```

---

### Task 2: `playbook_combo_trust` query

**Files:** Modify `crates/senseid/src/db/pg_store.rs`

- [ ] **Step 1: Write the failing DB test** (mirror the `playbook_tests` module + `PgStore::connect_test()`; pre-clean like the §9 tests)
```rust
#[tokio::test]
async fn playbook_combo_trust_counts_ftr() {
    let Ok(pg) = PgStore::connect_test().await else { return; };
    pg.execute_raw("delete from sensei.playbook_run where feature = 'trust-test'").await.ok();
    // two confirmed+attributed runs for (stable,bug,low, debug_flow): one ftr, one not → n=2, ftr=0.5
    for ftr in ["true", "false"] {
        pg.execute_raw(&format!(
            "insert into sensei.playbook_run (feature, lifecycle, intent, risk, playbook, rationale, confirmed, outcome_ftr) \
             values ('trust-test','stable','bug','low','debug_flow','t', true, {ftr})")).await.unwrap();
    }
    let (n, ftr) = pg.playbook_combo_trust("stable","bug","low","debug_flow").await.unwrap();
    assert_eq!(n, 2);
    assert!((ftr - 0.5).abs() < 1e-9);
    // empty combo → (0, 0.0)
    let (n0, f0) = pg.playbook_combo_trust("greenfield","ux","high","vibe").await.unwrap();
    assert_eq!(n0, 0); assert_eq!(f0, 0.0);
    pg.execute_raw("delete from sensei.playbook_run where feature = 'trust-test'").await.ok();
}
```

- [ ] **Step 2: Run + confirm fail** — `cargo test -p senseid playbook_combo_trust_counts_ftr 2>&1 | tail -5`.

- [ ] **Step 3: Implement** (mirror `playbook_combo_stats`)
```rust
pub async fn playbook_combo_trust(
    &self, lifecycle: &str, intent: &str, risk: &str, playbook: &str,
) -> Result<(i64, f64), String> {
    let row: (i64, f64) = sqlx_core::query_as::query_as(
        "SELECT count(*)::int8, coalesce(avg(outcome_ftr::int)::float8, 0.0)
           FROM sensei.playbook_run
          WHERE confirmed AND outcome_ftr IS NOT NULL
            AND lifecycle=$1::sensei.chunk_lifecycle AND intent=$2::sensei.chunk_intent
            AND risk=$3::sensei.chunk_risk AND playbook=$4"
    ).bind(lifecycle).bind(intent).bind(risk).bind(playbook)
     .fetch_one(&self.pool).await.map_err(|e| e.to_string())?;
    Ok(row)
}
```

- [ ] **Step 4: Run tests + clippy** — pass/skip; clean.

- [ ] **Step 5: Commit**
```bash
git add crates/senseid/src/db/pg_store.rs
git commit -m "feat(senseid): playbook_combo_trust query (n + FTR for one combo+playbook)"
```

---

### Task 3: `recommend_playbook` enrichment + `/sensei:intake` auto-confirm

**Files:** Modify `crates/senseid/src/api/handlers/playbook.rs`, `marketplace/plugins/sensei/commands/intake.md`

- [ ] **Step 1: Enrich the `recommend_playbook` handler.** After `rec` is computed + the run persisted + `opening_tone`/`when_to_use` fetched, add the trust check and include it in the response:
```rust
// Auto-select-on-trust: only low-risk chunks, only with proven FTR history.
let (auto_select, trust_n, trust_ftr) = if matches!(axes.risk, crate::playbook::Risk::Low) {
    match state.pg.playbook_combo_trust(
        axes.lifecycle.as_str(), axes.intent.as_str(), axes.risk.as_str(), &rec.playbook).await {
        Ok((n, ftr)) => (crate::playbook::is_trusted(axes.risk, n, ftr), n, ftr),
        Err(e) => { tracing::warn!(error=%e, "recommend_playbook: trust query failed — no auto-select"); (false, 0, 0.0) }
    }
} else { (false, 0, 0.0) };
```
Add to the returned `serde_json::json!({...})`: `"auto_select": auto_select, "trust": { "n": trust_n, "ftr": trust_ftr }`.

- [ ] **Step 2: Update the `/sensei:intake` command** (`marketplace/plugins/sensei/commands/intake.md`). In the recommend-and-confirm step, prepend an auto-select branch:
```markdown
4. Call `recommend_playbook(lifecycle, intent, risk, session_id=<session>)`. It returns `playbook`,
   `rationale`, `opening_tone`, and `auto_select` (+ `trust`).
   - **If `auto_select` is true:** skip the confirm — call `recommend_playbook(..., confirm="true")`
     to record the run, then tell the user: "Auto-selected **<playbook>** — reliable for this kind of
     chunk (FTR <trust.ftr> over <trust.n> runs). Say 'change' to pick a different playbook." Adopt
     `opening_tone` and proceed.
   - **Otherwise (recommend-and-confirm):** state the playbook + one-line `rationale`; on `risk=high`
     you MUST get explicit confirmation; on agreement call `recommend_playbook(..., confirm="true")`.
```
(Keep the rest of the procedure unchanged.)

- [ ] **Step 3: Verify.** `cargo build -p senseid 2>&1 | tail -3` (clean); `cargo test -p senseid -- --test-threads=1 2>&1 | tail -3` (green); `cargo clippy -p senseid --all-targets 2>&1 | tail -3` (clean). If a daemon is up: `curl -s localhost:7744/api/playbook/recommend -d '{"lifecycle":"stable","intent":"bug","risk":"low"}' -H 'content-type: application/json'` → response includes `auto_select` (false on a fresh DB). Confirm `intake.md` frontmatter still parses + `grep auto_select marketplace/plugins/sensei/commands/intake.md`.

- [ ] **Step 4: Commit**
```bash
git add crates/senseid/src/api/handlers/playbook.rs marketplace/plugins/sensei/commands/intake.md
git commit -m "feat(senseid,marketplace): auto-select-on-trust — recommend_playbook auto_select + /sensei:intake auto-confirm"
```

---

## Final verification (whole plan)

- [ ] `cargo test -p senseid -- --test-threads=1 2>&1 | tail -3` — green (incl. `is_trusted` + `playbook_combo_trust` tests).
- [ ] `cargo clippy -p senseid -p sensei-mcp --all-targets 2>&1 | tail -3` — clean.
- [ ] `recommend_playbook` returns `auto_select:true` only for a trusted low-risk combo; `false` for high-risk or thin/low-FTR history.
- [ ] `/sensei:intake` references `auto_select`; high-risk still requires confirm.
- [ ] Whole-plan review (subagent).

## Self-review notes (author)

- **Spec coverage:** `is_trusted` ✓ T1; `playbook_combo_trust` ✓ T2; `recommend_playbook` enrichment + command ✓ T3.
- **Type consistency:** `is_trusted(Risk, i64, f64)` matches the query's `(i64, f64)` return + the handler's `axes.risk`. Axis `as_str` labels feed the query's enum casts (identical to §9).
- **No new MCP tool / schema** — `auto_select` rides the existing `recommend_playbook` response; no DDL.
