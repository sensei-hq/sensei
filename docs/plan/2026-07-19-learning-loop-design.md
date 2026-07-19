---
name: §9 learning loop — design
date: 2026-07-19
status: design — approved in brainstorm 2026-07-19; implementation plan next
spec: docs/plan/operating-model.md §9 (Learning loop mechanics)
phase: Operating-model Phase 2 — front-door follow-up #3 (item 1 of the locked §9→auto-select→nudge/app sequence)
---

# §9 learning loop — design

Closes the front-door loop: **attribute a work-chunk's outcome back to the playbook that was
recommended, then adapt the rule set so the recommender compounds.** §9 (`operating-model.md`)
scopes this as *riding the existing analyzer + FTR loop* — "the playbook recommender becomes the
consumer of signals that currently have almost no consumer."

## Problem / context

`sensei.playbook_run.outcome` is never populated today. The analyzer already derives, per session,
`activity.sessions.{outcome (completed|corrected|blocked|partial|abandoned), ftr (=corrections==0),
corrections, turns}` at enrichment (L0 #66), and runs hourly via the analyzer scheduler. Nothing
consumes those signals for playbook recommendation. §9 adds a post-enrichment attribution + learning
stage that joins `playbook_run → its session`, records the outcome, and adapts `playbook_rules`.

## Decisions (brainstorm 2026-07-19)

1. **Full loop in one spec:** attribution **and** learning together.
2. **Signal = FTR** (`corrections==0`, the existing loop's currency, matches dojo-promote's efficacy);
   also snapshot the `session_outcome` enum onto the run for audit.
3. **Apply = hybrid:** auto-reweight **existing** rules' priority (bounded, self-correcting); a
   brand-new learned mapping is **proposed** (invisible to the resolver until accepted).
4. **Granularity = per `(lifecycle,intent,risk)` axes-combo × playbook**, global scope (matches the
   current rule structure). User/project-type dimensions deferred.
5. **Proposed = `enabled=false`** (reuse the existing column — the resolver already filters
   `WHERE enabled`; no new status enum). Accept flips `enabled=true`.
6. **Bounded auto-reweight of `priority`** via an immutable `base_priority` + a recomputed, clamped
   adjustment (idempotent, no runaway).

## 1. Data model (additive DDL — `dbd reconcile`)

- `sensei.playbook_run`: add **`outcome_ftr boolean`** (nullable) — the FTR snapshot at attribution.
  (`outcome text` already exists → stores the enum value.) Snapshotting onto the run means attribution
  survives activity-data GC of the source session.
- `sensei.playbook_rules`: add **`base_priority int`** (nullable; = the rule's original priority) so
  reweighting is `priority = base_priority + clamp(adj)` — recomputed each pass, never accumulated.
  `import_playbook_rules` sets `base_priority := priority` on insert; §9 backfills `base_priority :=
  priority` for any pre-existing null before its first reweight.
- Index: partial unique **`playbook_rules_learned_uq on (match_lifecycle, match_intent, match_risk,
  playbook) where source='learned'`** — makes learned-rule upsert idempotent (wildcards NULL are
  distinct per Postgres, which is fine: a learned rule always sets its matched axes).

## 2. Attribution (analyzer stage)

A new stage `attribute_playbook_outcomes(project)` in `analyze_project`, **after** session enrichment
(so `sessions.outcome`/`ftr` are set), before/alongside `generate_for_project`:

```sql
UPDATE sensei.playbook_run pr
   SET outcome = s.outcome::text, outcome_ftr = s.ftr
  FROM activity.sessions s
 WHERE pr.session_id = s.id
   AND pr.confirmed
   AND pr.outcome IS NULL
   AND s.outcome IS NOT NULL;
```
Idempotent (`outcome IS NULL` guard). Only **confirmed** runs with a live session are attributed
(MCP runs without a session, or unconfirmed recommendations, are excluded — they never ran).

## 3. Per-(axes×playbook) stats

Aggregate attributed runs into the input for the policy:

```sql
SELECT lifecycle, intent, risk, playbook,
       count(*) AS n,
       avg(outcome_ftr::int)::float8 AS ftr_rate
  FROM sensei.playbook_run
 WHERE confirmed AND outcome_ftr IS NOT NULL
 GROUP BY lifecycle, intent, risk, playbook;
```
For each axes-combo, the **recommended** playbook is the current resolver pick; its `ftr_rate` is the
baseline. Only combos/playbooks with `n >= MIN_SAMPLE` participate.

## 4. Learning policy — a PURE function

`fn learn(stats: &[ComboPlaybookStat], rules: &[Rule]) -> LearnPlan` where
`LearnPlan { reweights: Vec<(Uuid /*rule_id*/, i32 /*new_priority*/)>, proposals: Vec<LearnedRule> }`.
No IO — unit-testable. Constants (sensible defaults now; org-tunable = deferred):
`MIN_SAMPLE=5`, `FTR_DELTA=0.2`, `REWEIGHT_K=40`, `REWEIGHT_BOUND=20`, `REWEIGHT_TARGET_FTR=0.5`.

- **Reweight (existing rules):** for each enabled rule, aggregate the attributed runs whose axes
  **match the rule** (wildcard `NULL` match columns span all values of that axis) **and** whose
  `playbook` = the rule's playbook → the rule's observed `n` + `ftr_rate`. If `n>=MIN_SAMPLE`:
  `new_priority = base_priority + clamp(round(REWEIGHT_K * (ftr_rate - REWEIGHT_TARGET_FTR)),
  -REWEIGHT_BOUND, +REWEIGHT_BOUND)` where `REWEIGHT_TARGET_FTR = 0.5` (a fixed neutral midpoint —
  robust + degeneracy-free; only relative order matters to the resolver). (So the wildcard high-risk
  rule is scored on `spec_driven`'s FTR across *all* high-risk combos.) Deterministic from current
  stats ⇒ idempotent, bounded, self-correcting.
- **Propose (new mapping):** for an axes-combo where the **best** playbook `b` ≠ the recommended one
  `r`, and `b.n>=MIN_SAMPLE` and `b.ftr_rate - r.ftr_rate >= FTR_DELTA`: emit a `LearnedRule`
  (match = the exact combo, `playbook=b`, `priority` = the combo's current top priority + 1,
  `source='learned'`, `enabled=false`). Upserted via the partial unique index → re-runs update the
  pending proposal rather than duplicating.

## 5. Apply + accept path

- The analyzer stage runs `learn(...)`, then: `UPDATE playbook_rules SET priority=$new WHERE id=$id`
  for each reweight; `INSERT ... ON CONFLICT (learned uq) DO UPDATE` for each proposal (stays
  `enabled=false`).
- **Review + accept** (a proposed learned rule is invisible until accepted):
  - `list_playbook_rule_proposals()` (pg_store) → `GET /api/playbook/rule-proposals` + MCP tool
    `list_playbook_rule_proposals` — shows pending `source='learned', enabled=false` rules with the
    combo, proposed playbook, and the FTR delta that justified it.
  - `accept_playbook_rule(id)` (pg_store) → `POST /api/playbook/rule/{id}/accept` + MCP tool — sets
    `enabled=true`. (Reject = leave disabled / delete; a `reject` verb is a trivial follow-up.)
  - Dōjō is the natural cross-team owner of this review (governance plane) — deferred; the endpoints
    make it possible now.

## 6. Local-model read (the bonus §9 unlocks)

Attributed data makes the heterogeneous-execution metric real (see
`project_heterogeneous_execution_router`): FTR grouped by how the chunk was classified.
`GET /api/playbook/model-stats` + pg_store query:
```sql
SELECT classified_by, model_fallback, count(*) n, avg(outcome_ftr::int)::float8 ftr_rate
  FROM sensei.playbook_run WHERE confirmed AND outcome_ftr IS NOT NULL
 GROUP BY classified_by, model_fallback;
```
Answers "was the local model good enough to classify vs the heuristic/cloud" — the first real datapoint.

## 7. Where it runs

`analyze_project` (crates/senseid/src/tasks/handlers/analyze.rs), a new stage after enrichment +
`derive_signals`, before `generate_for_project`. Hourly via the existing analyzer scheduler; fully
idempotent (attribution guard + deterministic reweight + upsert proposals). Per-project pass; the
stats/policy are global (rules are global today) so the stage computes once per tick, not per project
— run it in the scheduler's global-pass set (alongside `AggregateCorrections` etc.), not per-project.

## Units & interfaces (isolation)

| Unit | Responsibility | Interface | Depends on |
|---|---|---|---|
| DDL | `outcome_ftr`, `base_priority`, learned unique index | schema | dbd |
| attribution | populate `playbook_run.outcome`/`outcome_ftr` from sessions | one UPDATE (pg_store) | DDL |
| stats | per-combo×playbook FTR + baseline | pg_store query → `Vec<ComboPlaybookStat>` | DDL |
| **`learn` (pure)** | stats + rules → reweights + proposals | `fn(&[stat], &[Rule]) -> LearnPlan` | nothing (pure) |
| apply | UPDATE priorities; UPSERT proposals | pg_store | learn, DDL |
| accept surface | list/accept proposals | pg_store + endpoints + MCP tools | DDL |
| model-stats | FTR by classified_by | pg_store query + endpoint | DDL |
| analyzer stage | orchestrate attribution→stats→learn→apply | global-pass task | all above |

## Testing

- **`learn` (pure):** reweight direction (good FTR → priority up, bad → down) + bound clamp +
  idempotence (same stats → same priorities); propose-on-delta (best≠recommended, delta≥0.2, n≥5) +
  no-op below min-sample + no-op when recommended already best. One assertion per case.
- **Attribution:** a confirmed run + enriched session → outcome/outcome_ftr populated; unconfirmed or
  session-less runs untouched; re-run is a no-op.
- **DDL:** columns + partial unique index apply; `import_playbook_rules` sets `base_priority`.
- **Upsert idempotency:** two passes over the same stats → one proposal row, updated not duplicated.
- **Accept path:** propose (enabled=false, resolver ignores it) → accept → enabled=true → resolver
  now returns it. MCP round-trip for list/accept.
- **model-stats:** FTR grouped by classified_by returns expected buckets.

## Scope / deferred

**In:** the 2 columns + index; attribution stage; stats; the pure `learn` policy (reweight + propose);
apply; list/accept endpoints + MCP tools; the model-stats read; wiring into the analyzer global pass.
**Out (own follow-ups):** user/project-type attribution granularity; Dōjō federation of learned rules +
a review UI; auto-accept on very strong evidence; composite/enum-weighted scoring; org-tunable
thresholds (constants now); a `reject` verb (leave-disabled suffices).

## Open questions (for plan)

- Exact analyzer global-pass registration point + task enum variant name — pin against
  `analyzer_scheduler.rs`'s existing global-pass set (`AggregateCorrections`, …) in the plan.
- `REWEIGHT_TARGET_FTR` is a fixed 0.5 midpoint (org-tunable later); revisit if domains show a systematically low/high baseline FTR where a data-relative baseline would differentiate better.
