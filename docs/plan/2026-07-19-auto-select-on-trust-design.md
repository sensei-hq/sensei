---
name: Auto-select-on-trust — design
date: 2026-07-19
status: design — approved in brainstorm 2026-07-19; plan next
spec: docs/plan/operating-model.md §3.3 ("low-risk chunks may auto-select once trust is established")
phase: Operating-model Phase 2 — front-door follow-up #3 (item 2 of §9→auto-select→nudge/app)
---

# Auto-select-on-trust — design

Low-risk chunks **auto-confirm** the recommended playbook (skip the human confirm) once §9's
outcome history shows the recommendation is reliable. Builds directly on §9 (just shipped):
`playbook_run.outcome_ftr`, the per-combo FTR data, `recommend_playbook`, the `/sensei:intake`
recommend-and-confirm flow.

## Decisions (brainstorm 2026-07-19)

1. **Trust = live-derived from §9 stats** (not a persisted flag): at recommend time, check the chosen
   playbook's FTR for the chunk's exact combo. No new schema; always current.
2. **On by default, announce-and-proceed:** when trusted, auto-confirm + announce (reversible); the
   strict gating makes it fire only when very safe. **High-risk always keeps the human confirm.**

## Trust decision (pure)

`fn is_trusted(risk: Risk, n: i64, ftr: f64) -> bool` = `risk == Risk::Low && n >= TRUST_MIN_SAMPLE &&
ftr >= TRUST_FTR`. Constants: `TRUST_MIN_SAMPLE = 10`, `TRUST_FTR = 0.8` — deliberately stricter than
§9's learn thresholds (5 / —), since skipping human oversight demands more evidence than a reweight.
Pure + unit-testable; lives in `crates/senseid/src/playbook.rs` next to `learn()`.

## Trust data (pg_store)

`playbook_combo_trust(lifecycle, intent, risk, playbook) -> Result<(i64 /*n*/, f64 /*ftr*/), String>`
— a focused aggregate over `sensei.playbook_run` (mirrors `playbook_combo_stats`, one combo+playbook):
```sql
SELECT count(*)::int8, coalesce(avg(outcome_ftr::int)::float8, 0.0)
  FROM sensei.playbook_run
 WHERE confirmed AND outcome_ftr IS NOT NULL
   AND lifecycle=$1::sensei.chunk_lifecycle AND intent=$2::sensei.chunk_intent
   AND risk=$3::sensei.chunk_risk AND playbook=$4;
```
Returns `(0, 0.0)` when there's no history → `is_trusted` → false.

## `recommend_playbook` enrichment

After the rule is picked and the run persisted, if `risk == Low`: fetch `playbook_combo_trust(axes,
rec.playbook)`, compute `auto_select = is_trusted(risk, n, ftr)`, and add to the JSON response:
`auto_select: bool` + `trust: { n, ftr }` (for the announce copy). For `risk == High` the trust query
is skipped entirely and `auto_select` is `false`. Advisory: the daemon reports it; the command acts.

## `/sensei:intake` (command procedure)

After calling `recommend_playbook`: if the response's `auto_select` is `true`, skip the clarifying
confirm — call `recommend_playbook(…, confirm="true")` to record the confirmed run, then **announce**:
"Auto-selected **<playbook>** — it's been reliable for this kind of chunk (FTR <ftr> over <n> runs).
Say 'change' to pick a different playbook." Adopt the `opening_tone` and proceed. Otherwise, the
existing recommend-and-confirm step (unchanged). `auto_select` is only ever true for low-risk, so
high-risk always keeps the human in the loop.

## Units & interfaces (isolation)

| Unit | Responsibility | Interface | Depends on |
|---|---|---|---|
| `is_trusted` (pure) | the trust gate | `fn(Risk, i64, f64) -> bool` | nothing |
| `playbook_combo_trust` | one combo+playbook FTR | pg_store query | §9 DDL (`outcome_ftr`) |
| `recommend_playbook` enrichment | set `auto_select`/`trust` | handler → response | is_trusted, the query |
| `/sensei:intake` | honor auto_select (auto-confirm + announce) | command `.md` | recommend_playbook response |

## Testing

- **`is_trusted` (pure):** low+n≥10+ftr≥0.8 → true; high-risk → false; n<10 → false; ftr<0.8 → false;
  boundary (n==10, ftr==0.8) → true.
- **`playbook_combo_trust`:** seed confirmed runs with known FTR → returns the expected (n, ftr);
  empty history → (0, 0.0).
- **`recommend_playbook`:** a trusted low-risk combo → `auto_select:true`; an untrusted / high-risk
  combo → `auto_select:false`.

## Scope / deferred

**In:** `is_trusted` + consts; `playbook_combo_trust`; `recommend_playbook` `auto_select`/`trust`
enrichment; `/sensei:intake` auto-confirm-and-announce. **Out:** org-tunable thresholds; per-user /
per-project trust; a global off-switch (inherently gated by strict trust + risk=low + announced/
reversible); any new MCP tool (auto_select rides the existing `recommend_playbook` response).
