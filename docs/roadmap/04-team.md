# Roadmap — Team plane (T1–T5)

The Dōjō: core (T1), consoles (T2), project planning (T3), billing/seats (T4).
Status legend: [`README.md`](README.md).

> Per-module statuses are filled in Phase 3 from the legacy coverage map
> (`../history/features/coverage-map.md`) and the plan gap analysis
> (`../history/plan/`). Structure is final.

| ID | Module | One-line "done" | Status |
|---|---|---|---|
| T1 | Dōjō Core | Knowledge travels up, is governed, and comes back down | 🚧 Phase 3 |
| T2 | Dōjō Consoles | All four roles (developer · maintainer · admin · lead) have a live console | 🚧 Phase 3 |
| T3 | Dōjō Project Planning | A project plans against its tracker and gates on code health | 🚧 Phase 3 |
| T4 | Billing & Seats | A private shared Dōjō can be subscribed and metered | 🚧 Phase 3 |
| T5 | Releases, DORA & Release Gates | Deployments are read from CI, the four keys are computed, and a release that would ship a regression is held | ⛔ not built — [09-dojo-expansion](09-dojo-expansion.md) Wave D |

## Known gaps affecting this plane

| Gap | What | Priority |
|---|---|---|
| T-1 | **T3's tracker adapters (Jira / ADO) exist only in the mockup — nothing is built**; 1 of 7 gating targets buildable today, 2 need scoped reads, 4 blocked on #156 | P1 |
| T-2 | join/membership flow: magic-link creates `auth.users` but no membership; "Join" dead → every console surface 403s | P1 |
| F-3 | Dōjō sync never worked (the plan's own record: "Dōjō sync never worked" — reorg §12) | P1 |

Full register: [`06-gaps.md`](06-gaps.md).

## Sequencing

T2's consoles are gated on T-2 (membership flow). T3's spec (reorg §9) is ready
to be written — the gate/improvement split and own-baseline ratchet (reorg
§9.3–9.4) are its fairness model. T4 rides the business model in `../prd/01-vision.md`
(free where public or personal).