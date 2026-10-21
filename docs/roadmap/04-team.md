# Roadmap — Team plane (T1–T5)

The Dōjō: core (T1), consoles (T2), workspace planning (T3), billing/seats (T4).
Status legend: [`README.md`](README.md).

> Per-module statuses are filled in Phase 3 from the legacy coverage map
> (`../history/features/coverage-map.md`) and the plan gap analysis
> (`../history/plan/`). Structure is final.

| ID | Module | One-line "done" | Status |
|---|---|---|---|
| T1 | Dōjō Core | Knowledge travels up, is governed, and comes back down | 🚧 Phase 3 |
| T2 | Dōjō Consoles | All four roles (contributor · maintainer · lead · admin) have a live console | 🚧 Phase 3 |
| T3 | Dōjō Workspace Planning | A workspace plans against its tracker and gates on code health | 🚧 Phase 3 |
| T4 | Billing & Seats | A private shared Dōjō can be subscribed and metered | 🚧 Phase 3 |
| T5 | Releases, DORA & Release Gates | Deployments are read from CI, the five DORA keys are computed, and a release that would ship a regression is held | ⛔ not built — [09-dojo-expansion](09-dojo-expansion.md) Wave D |

## Known gaps affecting this plane

| Gap | What | Priority |
|---|---|---|
| T-1 | **T3's tracker adapters (Jira / ADO) exist only in the mockup — nothing is built**; 1 of 7 gating targets buildable today, 2 need scoped reads, 4 blocked on #156 | P1 |
| ~~T-2~~ | ~~join/membership flow dead~~ — **resolved (#307 A3, 2026-10-21):** an admin invites a member or a viewer (7e547a19); accepting makes the membership or the dōjō viewer | — |
| F-3 | Dōjō sync: **works against the local dōjō** (2026-10-20/21: repositories, metrics, graph and relay; #301/#299 verified there); **production unverified** until a daemon is enrolled with dojo.sensei-hq.com. #184 (`sync_status`) still open | P1 |

Full register: [`06-gaps.md`](06-gaps.md).

## Sequencing

T2's consoles were gated on T-2 (membership flow), resolved in #307 A3. T3's spec (reorg §9) is ready
to be written — the gate/improvement split and own-baseline ratchet (reorg
§9.3–9.4) are its fairness model. T4 rides the business model in `../prd/01-vision.md`
(free where public or personal).