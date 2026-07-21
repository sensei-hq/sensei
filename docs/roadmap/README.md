# Roadmap

Purpose: phases, sequencing, and current status; audience: product owner.

This is an index, not a copy — the detailed, living sources are
[`../plan/README.md`](../plan/README.md) (the gap analysis that drives sequencing)
and [`../spec/EXECUTION-PLAN.md`](../spec/EXECUTION-PLAN.md) (the locked execution plan).

- **[phases.md](phases.md)** — the six-phase roadmap (MVP → relay → analytics → chat → payments → orchestrator) with goals, includes, and exit criteria. Detailed phase 1–3 stories: [`../plan/2026-07-20-phases-1-3-plan.md`](../plan/2026-07-20-phases-1-3-plan.md).

## Current status (distilled)

- 25+ screens are live; surface area is largely built (Observatory + project window essentially complete).
- The learning loop is the current focus, not more surface area: capture → graph → analyze are fresh; learn/deliver/measure are the closing work.
- FTR (first-turn resolution — the fraction of sessions whose first attempt landed without a correction) is the north-star metric the roadmap is organized around.
- The FTR loop closed and memory promotion unblocked as of 2026-07-15 — the reinforcement path from a proven recommendation back to its source memory now works end-to-end.
- Doc-drift signal quality, semantic search + context-pack, and command-governance preferences have shipped or substantially progressed on `develop`.
- Front-door intake and adaptive playbooks are shipping on `develop`; not yet released to `main`.
- Remaining net-new surfaces (solution segment, bootstrap splash, consolidation screen, insights-reasoning drawer) are intentionally lowest priority — sequenced after the loop closes, since each is only as good as the data behind it.
- External-blocked work (collective-intelligence, Dōjō federation) is substantially built but paused pending a remote Dōjō server and an opt-in decision.

This page is living — updated as workstreams land.
