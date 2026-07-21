---
name: Phases
type: roadmap
updated: 2026-07-20
---

# Phases

The product ships as one loop, in phases. The guiding rule: **build the loop —
free where the work is local or public, paid where it is private and shared.**
The MVP is the local loop working end to end; each later phase adds a plane on
top without breaking the one below.

Detailed, buildable stories for phases 1–3 (with acceptance criteria + Playwright
and `sensei`-DB verification) live in
[`../plan/2026-07-20-phases-1-3-plan.md`](../plan/2026-07-20-phases-1-3-plan.md).

| Phase | Goal | Includes | Status |
|---|---|---|---|
| **P1 · MVP** | The local loop works end to end — observe, understand, and govern your own projects, run by hand | bootstrap + scanning ([01-setup](../features/01-setup.md)) · projects + the project window ([04-project](../features/04-project.md)) · governance plane / rules resolution ([05-governance](../features/05-governance.md)) · playbooks **manual via `sensei:` commands** ([04-project · Working style](../features/04-project.md#working-style--the-operation-manual)) | Mostly built; **project-window bug is the top blocker** |
| **P2 · Relay** | Run and supervise work away from the keyboard | [06-relay](../features/06-relay.md) — remote plan · execute · status · HITL | P0–P4 on `develop`, flags off; not on `main` |
| **P3 · Analytics / insights / guidance** | Turn observation into useful, measured guidance | analyzer · insights · recommendations · impact / verdicts · FTR metrics · in-the-moment MCP guidance ([03-observatory](../features/03-observatory.md)) | Analyzer + insights + recs largely built; traceability / atlas partial |
| **P4 · Chat guidance** | User guidance through a chat interface | a chat surface over the graph + memories + guidance | Not started |
| **P5 · Payments** | Monetize team coordination | [07-pricing](../features/07-pricing.md) + Stripe billing + per-seat metering + plan selection at dōjō creation | Mock only |
| **P6 · Custom orchestrator** | sensei drives multi-model agentic execution itself | Planner→Builder→Judge "repeat until it works" loop · gateway mix-of-models (oauth + api-key) MoE · opencode base · skilled agentic coordinator ([remote module](../design/remote.md)) | Future |

## Exit criteria

| Phase | Done when |
|---|---|
| P1 | Fresh install → scan → the project window opens → real memories / patterns / insights surface from the `sensei` DB → rules resolve live; playbooks run via `/sensei:` commands. |
| P2 | A gate can be approved and a run nudged from phone / console; the daemon run-engine drives a plan (dev-verified, flags on). |
| P3 | Insights + impact are surfaced from real data and are genuinely useful; guidance reaches the assistant on the first turn. |
| P4 | A user can converse with sensei about a project and get grounded answers. |
| P5 | A team can subscribe and be metered per active contributor; individuals / OSS stay free. |
| P6 | sensei executes a chunk through its own Planner→Builder→Judge orchestrator using a mix of models via the gateway. |

## Notes

- **P6 simplifies the depth problem.** The Planner→Builder→Judge loop catches gaps
  during execution (the Judge re-plans), so a plan no longer has to be
  exhaustively deep before an autonomous run — it relaxes the plan-depth bar.
- **Playbooks are manual in the MVP** (driven by `sensei:` commands); auto-select
  and the outcome-learning loop arrive with P3.
- **Nothing is built before its data exists.** Surfaces backed by real data (code
  graph, libraries, tool-usage, commands) are built and verified now; thin or
  empty surfaces (memories, impact, federation) get honest empty states until the
  data accrues — see the plan doc's real-data map.
</content>
