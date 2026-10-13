# Team plane — the Dōjō

> Team = T1–T4. The cross-cutting team layer: knowledge travels up, is governed,
> and comes back down — the org boundary is the Dōjō membership. Status:
> [`../../../roadmap/04-team.md`](../../../roadmap/04-team.md).

| ID | Module | One-line "done" | Spec |
|---|---|---|---|
| T1 | Dōjō Core | Knowledge travels up, is governed, and comes back down | 🚧 Phase 2 |
| T2 | Dōjō Consoles | All four roles (developer · maintainer · admin · lead) have a live console | 🚧 Phase 2 |
| T3 | Dōjō Project Planning | A project plans against its tracker and gates on code health | 🚧 Phase 2 |
| T4 | Billing & Seats | A private shared Dōjō can be subscribed and metered | 🚧 Phase 2 |

## What this plane owns

Memberships, routing, preview and the universal strip (T1); the role consoles
(T2, old roles live in [`02-objectives.md`](../../02-objectives.md) §Dōjō — developer ·
maintainer · org admin · **lead** — plus the new manager/client viewpoints);
project planning against external trackers (Jira / ADO read-only) with
**delta-scoped, ratchet-fair quality gates** (T3, reorg §9); and the free-where-public
business model with payment for private shared coordination (T4).

## Reading order

T1 → T2 → T3 → T4. T3 is the newest surface; read [`../../../reorg.md`](../../../reorg.md) §9
first — 2 of its 7 gating targets are buildable today, 2 need scoped reads, 3
are blocked on the architecture-metrics family (#156).

## Customer

The Team plane serves every persona this pass added — **manager / delivery-lead**
and **client / project stakeholder** — as well as the maintainer, admin and lead
roles. It is the payload for the platform's billing question.

## Legacy sources

Folded from `features/05-governance.md`, `features/07-pricing.md`,
`features/dojo/`, `spec/screen/dojo-*.md`,
`spec/pipeline/{dojo-lifecycle,governance}`,
`spec/dojo/{dojo-auth-provisioning.md,…}`,
`architecture/{dojo,dojo-deployment,entity-access-model}`,
`journeys/dojo.md`, `mockups/Sensei/lib/dojo2/` (visual source — never edited).