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

## Decisions (Rev 1) — RESOLVED by user 2026-06-24
- ✅ **Schema fork**: EXTEND the DDL (memory `references` jsonb + structured fields; recommendation `based_on` links + reconciled `action_type`). Derived data, not presentation.
- ✅ **Enum reconciliation**: go ahead — canonical sets chosen below.
- ✅ **maturity**: DERIVE early/mature (don't replace the enum wholesale; compute the binary signal from enriched-session count + insight presence).

### Canonical enums (chosen)
- `recommendation.action_type` (union, UI-leaning): `promote_pattern | create_agent | write_skill | archive_memory | enrich_memory | cross_project | revise_rule | audit_stale`.
- `memory.type` keeps the structural kind (`pattern|convention|preference|decision|continuity|question`); add a separate `category` (`correctness|convention|pattern|preference`) for the UI's anatomy grouping, OR fold into `type` — to finalize at build time, leaning two fields (type=structural, category=nature).

---

## Revision 2 (2026-06-24) — updated mockups + journey maps

Re-surveyed the refreshed mockups (Dōjō console + new in-app Dōjō, hive **site**, project-logs, inference settings) and the journey maps (main app, Dōjō, flow walkthrough). The Rev-1 core gap (L2/L3 unwired + memory/recommendation shape) **still stands and is still the top priority**. New surfaces + bigger picture below.

### New surfaces the daemon must back

1. **Inference settings ⇄ gateway config (ties directly to #76/#79).** `wiz-inference.jsx`/`setup-data.js` model inference as **roles** with fallback chains: `inference | consolidation | embedding | voice | image` (each = primary→secondary→tertiary), plus per-provider config (`detected`, `configured`, `envVar`, `kind: cloud|local|custom`) and per-local-model **pull status**. This is the UI for the table-driven gateway I just built. Gaps: roles `voice` + `image` have no seed chains; there's no **config read/write API** for routers/models/role-chains and no **model-pull-progress** endpoint. Map: inference→reasoning/chat, consolidation→reasoning, embedding→embed; add voice (audio) + image chains.
2. **Project Logs / diagnostics (#39).** `project-logs.jsx` + `PLOG_SESSIONS`: diagnostic runs with `trace[]` = `{ts, action_type: check|resolve|instruct, step, desc, cmd, exit, out, err, ms, ok, fix_attempted, fix_approach, fix_ok}`, `system_info{os,arch,ram_gb,cpu_cores}`, `module: bootstrap|session|scan|wizard`, `outcome`, + an anonymized **GitHub-issue export**. `/api/logs` ingest exists but not this rich trace shape/export. Ties to the sensei-logger crate.
3. **Dōjō governance — big expansion (federation client ⇄ hive server).** Journey defines a full pipeline: multi-org membership; **project→org binding** (forward-only routing history); **upstream share with anonymization/dereferencing + redaction preview**; **contribution status timeline** (queued→triaged→approved/declined→adopted); **downstream receive** with **scope-ladder conflict resolution** (org›team›global›personal, both rules shown); maintainer triage (conflict diffs, dedup ≥0.9 auto / 0.75–0.9 flagged, approval thresholds, distribution **dry-run**); **client confidentiality** (universal anonymization, immutable **audit log**, **leak-guard** + quarantine/retract); observability (contribution/approval/adoption rates). Much exists in skeleton (knowledge sources/proposals/promotion/namespaces/enforcement) but the anonymization+audit, status timeline, scope-ladder, leak-guard, and dry-run are not built. **Architectural split to confirm: which is daemon (client) vs sensei-hive (server)?**
4. **Hive site (`hq/site.jsx`).** Public product/portfolio/philosophy site (Sensei/DBD/Rokkit/Kavach + incubating + principles). Static content — **minimal daemon impact** (no derived data), unless collective stats shown there are dynamic.
5. **Bootstrap/provisioning telemetry.** Journey wants live setup progress (step, %, size, time), component detect+remediate, "calm when green," multi-pass setup. Health checks exist; **model-pull progress streaming** (ties to the #79 provisioner) does not.
6. **Today's koan / one-decision triage.** One focal koan/day + Apply|Review|Dismiss per insight → reinforces the L2 recommendation work (a ranked pick + a primary action per recommendation).

### New open decisions (to resolve before "daemon covers all gaps")
- **Inference roles ↔ gateway chains**: confirm roles == named chains; add `voice` + `image` chains; and build the **inference-config read/write API + model-pull-status API** the settings UI needs (a real chunk, building on #76/#79). 
- **Daemon vs sensei-hive split for governance**: the journey's triage/approval/distribution/audit is largely **server-side (hive)**; the daemon does anonymize-before-send, binding, status-poll, downstream-receive+conflict-resolution, local audit. Confirm the boundary + whether hive-server work is in scope here or in the `sensei-hive` repo (raise as a cross-repo issue per house rule).
- **Project-logs (#39)**: build the rich `trace` capture + anonymized export now, or defer? (Diagnostic value is high; it's self-contained.)
- **Provisioning telemetry**: add pull-progress events (small) alongside the #79 provisioner trigger when wired.

### Updated build order (daemon-side, supersedes Rev 1)
1. Resolve the shape forks (memory `references` + recommendation taxonomy/`based_on`) — small DDL.  ← decisions made; ready.
2. **L2 Generator (F4/#69)** → recommendations + learned memories (+ koan pick). Lights up the most UI.
3. **L3 maturity (F6/#71)** — early/mature + watched/target.
4. **Inference-config + pull-status APIs** (back the inference-settings UI; add voice/image chains).
5. Pattern effectiveness (ftrDelta + lifecycle) + corrections aggregation.
6. **L2 consolidation (F5/#70)** + impact reasoning_traces.
7. **Project-logs (#39)** rich trace capture + export (independent; can slot earlier).
8. Dōjō governance expansion — **after the daemon/hive split is confirmed** (likely a cross-repo effort).
9. Contract hygiene (snake/camel, outcome vocab) before UI wiring.

---

## Revision 3 (2026-06-24) — scope LOCKED by user

- **No anonymization / dereferencing / redaction / leak-guard.** By construction, generated memories/patterns **never reference real code** — they carry only synthetic/suitable examples. So "see what's shared" is fine; there is **no** UI redaction-preview or anonymization pipeline to build. Drop all of that from the Dōjō scope (Rev-2 item 3's confidentiality machinery is OUT). The mockups already reflect this.
- **Standalone Sensei first; Dōjō/hive deferred.** Focus all daemon work on the single-user app surfaces. The Dōjō/federation/governance pieces (and the daemon-vs-hive split question) start **after standalone is complete** — not now.
- **Inference roles:** `inference → reasoning` chain. **Add `voice` + `image` chains** to the seed (voice = `audio` capability; image generation still needs `model_capability` `image` per #77). Build the inference-config + model-pull-status APIs as part of standalone.
- **Logs:** make them **richer** — today only the `bootstrap` module is captured; add `session`/`scan`/`wizard` modules + the rich `trace` shape + anonymized GitHub-issue export (#39). **Add a TTL/retention for logs** (ties to #74 retention/pruning).
- **Website `/sensei` site:** updated mockup (focused sections, covers Dōjō) → tracked in **#81**, done **after** the daemon is complete; Dōjō section held until Dōjō ships.

### Standalone-completion build order (the active plan)
1. **Shape-forks DDL** — memory `references` (good/bad example path:line — synthetic, `related[]`, `evidence[]`, `doc`) + what/because/consequence split + `scope.level`; recommendation reconciled `action_type` + `based_on{patterns,memories,corrections}`; (decisions made).
2. **L2 Generator (F4/#69)** → recommendations + learned memories (+ today's-koan ranked pick). Biggest UI unlock.
3. **L3 maturity (F6/#71)** — derive early/mature + sessions watched/target.
4. **Inference**: add `voice`+`image` chains; inference-config + pull-status APIs (on #76/#79).
5. **Pattern effectiveness** (ftrDelta + emerging/adopted/promote-candidate lifecycle) + **corrections aggregation**.
6. **L2 consolidation (F5/#70)** + impact `reasoning_traces` + extra deltas (uses the embedded `reasoning` chain).
7. **Project-logs (#39)** richer capture (all modules + trace) + **log TTL (#74)** + anonymized export.
8. **Contract hygiene** (snake/camel, outcome vocab) before UI wiring.
9. **DEFERRED:** all Dōjō/hive/federation governance + the daemon-vs-hive split + website Dōjō section (#81) — after standalone.
