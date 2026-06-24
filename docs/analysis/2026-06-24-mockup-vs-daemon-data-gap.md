---
title: Mockup data-shapes vs daemon API/derivation — gap analysis
description: Cross-reference of what the redesigned UI mockups consume vs what the daemon APIs expose and the analysis pipeline derives, to find daemon-side structure gaps before UI↔daemon wiring.
type: analysis
status: analysis
created: 2026-06-24
references:
  - docs/mockups/Sensei/
  - crates/senseid/src/api/
  - crates/senseid/src/tasks/handlers/analyze.rs
  - docs/blueprints/2026-06-22-session-analyzer.md
---

# Mockup ↔ Daemon data gap analysis

Three-way inventory: **UI needs** (mockups) ↔ **API exposes** (HTTP handlers) ↔ **pipeline derives** (L0/L1/L2/L3 + tables). Goal: confirm we're *capturing/deriving the correct structure* before wiring the UI.

## Verdict in one line

The **read primitives** (sessions, FTR, code graph, patterns, knowledge/governance, federation) are solid and largely match the UI. **Every UI surface that shows _learning_ — Recommendations inbox, Memories/Learnings, Impact reports, Consolidation, pattern effectiveness, project maturity — is backed by tables with no writer (L2/L3 unwired).** So those screens would render blank, and where writers *do* exist the captured **shape is thinner than the UI needs**.

---

## What's aligned (no work)

| UI surface | API | Derivation | Status |
|---|---|---|---|
| Observatory FTR strip / sparklines | `/api/observatory/ftr-daily`, `/api/projects/{id}/ftr-daily` | L0 `activity.sessions.ftr` + `project_ftr_metrics` view | ✅ live |
| Sessions digest (id, project, duration, turns, corrections, ftr, outcome, tool usage) | `/api/sessions`, `/api/projects/{id}/sessions` | L0 enrichment | ✅ live (minor: outcome vocab differs, see below) |
| Patterns list (name, anti, confidence, occurrences, sample) | `/api/patterns/{project}`, `/api/projects/{id}/patterns` | L1 `detected_patterns` | ✅ live (missing ftrDelta/status, see below) |
| Code graph (nodes/edges/communities/duplicates) | `/api/graph/*`, `/api/patterns/{p}/duplicates` | indexer | ✅ live |
| Dōjō triage queue / candidates / promotion | `/api/knowledge/proposals`, `/promotion-candidates`, `/sources` | governance + federation | ✅ largely covered |
| Hotspots / churn | `/api/projects/{id}/hotspots` | L0/L1 | ✅ live |

---

## Gaps — daemon-side work, by priority

### P0 — L2 Generator is not wired (the big one)
The blueprint's **F4 (#69 heuristic) + F5 (#70 consolidation)** never landed. `inference.recommendations`, `inference.reasoning_traces`, and `sensei.memories(origin='learned')` are **empty** — only federation + manual API write memories; nothing writes recommendations. L1 detects patterns (churn / correction-prone / rule-candidates) but **nothing maps them into recommendations or learned memories.**

UI blank without it: Recommendations/"do first" inbox, Memories/Learnings (Anatomy), Impact, Consolidation, Corrections-with-suggestion.

→ **Build the L2 Generator**: read `detected_patterns` + correction/principle signals → write `inference.recommendations` + `sensei.memories(origin='learned')`. Consolidation tier (F5) uses the gateway `reasoning` chain (now embedded-first, #79) → write `inference.reasoning_traces`.

### P0 — Build it to the *right shape* (structural mismatches to resolve first)
Where writers exist, the schema is thinner than the UI. Reconcile before/while building L2:

**Memory** — UI `Memory` wants structured fields the DB lacks:
- `references: { good_example(path:line), bad_example(path:line), pattern, evidence[session_ids], related[memory_ids], doc }` — DB has only free-text `content` + a single `session_id`. **No structured references.**
- what / because / consequence **split** — DB has `title` + `content` + `impact` (maps loosely; UI renders three distinct parts).
- `scope.level` = global|stack|project|module + `stack[]`/`modules[]`/`taskTypes[]` — DB has `scope` + `scope_filter` (less structured).
- `category` (correctness|convention|pattern|preference) vs DB `type` (pattern|convention|preference|decision|continuity|question) — **enum mismatch**.
- `lastRelevant` timestamp — not captured.

**Recommendation** — UI vs DB **action taxonomy diverges**:
- UI `kind`: promote-pattern | create-agent | write-skill | archive-memory | enrich-memory | cross-project
- DB `action_type`: promote_pattern | create_persona | enable_skill | audit_stale | revise_rule | cross_project
- UI `basedOn: { patterns[], memories[], corrections[] }` (links to ids) vs DB `evidence: [{session_id, file, description}]`. UI wants provenance links to the pattern/memory that triggered it; DB stores session/file evidence. **Both are useful — likely want both.**
- UI `targetKind`/`targetName` vs DB `action_detail{persona_name,pattern_id,skill_id,cwd}`.

→ **Decision needed (schema fork):** extend the DDL (memory `references` jsonb + reconcile `recommendation.action_type` enum + add `based_on` links), **or** keep the DB lean and have the API/handler transform to the UI shape. Recommend: add the structured columns (this is captured data the generator should produce, not view-only).

### P1 — Pattern effectiveness + lifecycle
UI `Pattern` wants `ftrDelta` ("18% better FTR when applied"), `kind` emerging|adopted|anti (DB only has `is_anti_pattern` bool), `status` promote-candidate|adopted|unclaimed|antipattern, cross-project `projects[]`, `memoryId` link. None derived. → correlate pattern adherence ↔ FTR; add emerging/adopted/promote-candidate classification; aggregate patterns across folders/projects.

### P1 — L3 project maturity
UI drives the whole Observatory on **early vs mature** + `firstSession { watched, target }` (sessions-watched-vs-target ~3). DB `projects.maturity` is discovery|active|maintenance|archived, **not auto-computed**, vocab mismatched. → **F6 (#71)**: compute early/mature from enriched-session count + insight presence; expose a maturity signal endpoint.

### P1 — Impact reports (verdict is partial)
`MeasureVerdicts` computes baseline/current FTR + verdict on accepted recs. But UI `ImpactReport` also wants `correctionsDelta`, `avgSessionDelta`, `toolUsageDelta{tool:pct}`, and `moeReasoning{ headline, body, models[{name,verdict,note}], consensus, suggestedRevision }` — the MOE panel verdicts map to `reasoning_traces`, **not written**. → extend verdict measurement with the extra deltas + write a reasoning trace per consolidation.

### P2 — Smaller structure gaps
- **Corrections view**: UI wants per-correction-text aggregation `{text, count, lastSeen, projects[], suggestion, memoryId?}`; L1 has folder-level correction-prone patterns with prompt snippets. → aggregate by recurring correction text + attach suggestion/memory link.
- **Consolidation candidates** (overlapping-memory merges) — UI `Consolidation` shape; no writer. Tied to L2.
- **Dōjō confidentiality**: `dereferenced` (client-identifier scrubbing) counters — not captured.
- **Doc traceability**: UI wants reference-level `{lineRef, quote, target{symbol,path}, status, expected, actual, diff, reason}`; `/api/graph/doc-drift` + `project_drift` view exist but the rich expected-vs-actual-signature diff may be thinner.
- **Response-contract hygiene**: endpoints mix `snake_case` and `camelCase`; UI fixtures lean camelCase. Pick one at the API boundary before wiring.
- **Outcome vocab**: UI uses shipped|abandoned; DB uses completed|corrected|blocked|abandoned. Decide the surface mapping.

---

## Recommended build order (daemon-side)

1. **Resolve the shape forks** (memory `references` + recommendation taxonomy/`based_on`) — small DDL + agreement on enums. Everything downstream depends on the target shape.
2. **L2 Generator heuristic (F4/#69)**: patterns/signals → recommendations + learned memories, in the agreed shape. Lights up the most UI at once.
3. **L3 maturity (F6/#71)**: early/mature + watched/target. Cheap; the Observatory hinges on it.
4. **Pattern effectiveness (ftrDelta + lifecycle)** + **Corrections aggregation**.
5. **L2 consolidation (F5/#70)** + impact reasoning_traces + extra deltas (uses the embedded `reasoning` chain from #79).
6. **Contract hygiene** (case + outcome vocab) as the last step before UI wiring.

## Open decisions for the user
- **Schema fork**: extend DDL for memory `references` + recommendation `based_on`/taxonomy, vs API-layer transform? (Recommend: extend DDL — it's derived data, not presentation.)
- **Enum reconciliation**: align `recommendation.action_type` and `memory.type`/`category` between UI and DB (pick the canonical set).
- **maturity vocab**: keep discovery|active|maintenance|archived and derive early/mature from it, or replace?
