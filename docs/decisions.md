---
name: Decisions log — sensei
updated: 2026-07-20
---

# Decisions

> Append-only. One entry per decision: date · decision · why · alternatives.
> The anti-rework memory — never re-derive a settled choice.
> The **design journey** (approaches tried, what worked, what didn't) lives in
> [`analysis/2026-07-24-design-journey.md`](analysis/2026-07-24-design-journey.md).

---

## 2026-07-24 — Auto-buildout decisions locked (P0–P5)

Locked with Jerry ahead of the phased unattended build-out. Full context:
[`analysis/2026-07-24-auto-buildout-readiness.md`](analysis/2026-07-24-auto-buildout-readiness.md) §4.

| Decision | Locked answer | Why / rejected |
|---|---|---|
| **Relay-first** | Build **relay supervision first** (P1): publish the plan as a relay run (phases→segments), daemon federates status → Dōjō, Jerry watches as `jerrythomas` + nudges; **no update in 5 min = stall**. Relay **STATUS** (safe), distinct from relay **DRIVE** (off). | Makes the whole unattended run watchable/nudgeable — solves "am I stuck?" without pings. Rejected: task-list-only visibility (too coarse for a long run). |
| **D-INJECT** | Governance rules are **pushed**, not pulled: resolve the repo's rules → inject into `additionalContext` at **SessionStart**, re-inject the **mandatory tier at PreCompact**, and inject into the **relay driver** for daemon runs. Mandatory/required always; advisory on-demand. | Root cause of "not sticky" = pull-only (`get_rules` reminder). Rejected: nudge-only (status quo, doesn't stick); SessionStart-only (drops after compaction). |
| **D-RULEPACK** | A rule pack is a **new `dojo.rule_packs` table + pack↔rule join** (area/scope/source/enforcement first-class). | Clean provenance + adopt-at-scope + maps to the designer's original LIB_PACKS model. Rejected: tag/collection on `shared_rules` (weaker metadata). |
| **D-COORD** | Coordinator **v1 = task-typed router at `driver_for` + gateway single-shot leg for cheap types + attribution ledger, drive OFF.** | Smallest useful superpowers-mirror. Rejected (v1): local ACP coding-agent + Planner→Builder→Judge loop (bigger, riskier → follow-up). |
| **D-GATE** | Blocking PreToolUse hook-gate stays **OFF** (activation = Phase-5 human gate). | Enforcement on live sessions is irreversible-ish; Jerry-gated. |
| **D-DRIVE** | `SENSEI_RUN_DRIVE` stays **OFF** the whole unattended run. | Creating/publishing runs is safe; driving a live agent is a human gate. |
| **D-SEED** | Invoke `dojo.seed_default_governance()` on the **Worker/Supabase** side + auto-register the global-dōjō pull `knowledge_source` on install. Prod-apply **gated**. | The seed has zero callers today ("reaches nobody"). |
| **D-ORIGIN** | Canonical provenance vocab = `authored | promoted | federated`; retire `remote`. | Doc/code mismatch (`remote` vs `federated`). |
| **D-STANCE-SCOPE** | Personal **stance** store is **user-scoped, not tenant-scoped**. | Personal governance follows the user with or without a dōjō. |
| **D-TIER3-DDL** | Tier-3 tables full-DDL-no-ALTER under `database/ddl/table/dojo/*.ddl`, `dbd deploy` against **e2e/local**; shapes proposed + confirmed as authored. Prod apply **gated**. | dbd is the schema tool; never Cloudflare-D1 (that's Decision-1). |
| **D-BILLING** | Billing = **schema + route only**; payment provider out of the unattended run until chosen. | No provider decided. |
| **D-CUTOVER** | `/console`→dojo2 redirect, `develop`→`main` merge, prod deploy, prod seed — **all human-gated (Phase 5)**. | Irreversible / trust-boundary. |
| **D-DECISIONS-HOME** | This file is canonical; per-feature → `features/<name>/decisions.md`. | The empty canonical slot; ADRs were misfiled. |

**Unattended boundary (standing):** safe-unattended = deterministic, local, reversible,
verifiable per chunk (DDL edits, Rust/TS on `develop`, tests, local browser-verify, dbd on
e2e/local, commits/pushes to `develop`). Human-gated = cloud/prod DB writes, Worker prod
deploy, secrets/VAPID/service-role, merge→main, cutover, gate activation, flipping
`SENSEI_RUN_DRIVE`, any payment-provider choice. The loop prepares + stages gated work,
writes the evidence, and waits.
