# Team plane — the Dōjō

> Team = T1–T5. The cross-cutting team layer: knowledge travels up, is governed,
> and comes back down — the org boundary is the Dōjō membership. Status:
> [`../../../roadmap/04-team.md`](../../../roadmap/04-team.md).

| ID | Module | One-line "done" | Spec |
|---|---|---|---|
| T1 | Dōjō Core | Knowledge travels up, is governed, and comes back down | [`README`](01-dojo-core/README.md) · [`dojo-core`](01-dojo-core/scenarios/dojo-core.feature) · [`dojo-core-gaps`](01-dojo-core/scenarios/dojo-core-gaps.feature) |
| T2 | Dōjō Consoles | All four roles (developer · maintainer · admin · lead) have a live console | [`README`](02-dojo-consoles/README.md) · [`dojo-consoles`](02-dojo-consoles/scenarios/dojo-consoles.feature) · [`dojo-consoles-gaps`](02-dojo-consoles/scenarios/dojo-consoles-gaps.feature) |
| T3 | Dōjō Project Planning | A project plans against its tracker and gates on code health | [`README`](03-dojo-project-planning/README.md) · [`dojo-project-planning`](03-dojo-project-planning/scenarios/dojo-project-planning.feature) · [`dojo-project-planning-gaps`](03-dojo-project-planning/scenarios/dojo-project-planning-gaps.feature) |
| T4 | Billing & Seats | A private shared Dōjō can be subscribed and metered | [`README`](04-billing-seats/README.md) · [`billing-seats`](04-billing-seats/scenarios/billing-seats.feature) |
| T5 | Releases, DORA & Release Gates | Deployments are read from CI, the four DORA keys are computed, and a release that would ship a regression is held | [`README`](05-releases-and-dora/README.md) · [`releases-and-dora`](05-releases-and-dora/scenarios/releases-and-dora.feature) |

## What this plane owns

Memberships, routing, preview and the universal strip (T1); the role consoles
(T2 — the four Dōjō roles live in [`02-objectives.md`](../../02-objectives.md) §Dōjō:
developer · maintainer · org admin · **lead**, alongside the manager and
client / project viewpoints);
project planning against external trackers (Jira / ADO read-only) with
**delta-scoped, ratchet-fair quality gates** (T3, reorg §9); the free-where-public
business model with payment for private shared coordination (T4); and release
pipelines, the four DORA keys and **release gates** that hold a release to the T3
targets over its change set (T5).

**Resolved 2026-10-21** ([roadmap/09](../../../roadmap/09-dojo-expansion.md)): external
systems (trackers, CI) are connected and owned by the **dōjō**, per project, and are
**read-only — the dōjō pulls**; **sensei ⇄ dōjō is two-way sync** for projects, teams,
roles and connections, with the dōjō the authority on conflict. The canonical role
ladder is `contributor < maintainer < lead < admin`.

## Reading order

T1 → T2 → T3 → T4 → T5 (T5 builds on T3's targets and integrations). T3 is the newest surface; read [`../../../reorg.md`](../../../reorg.md) §9
first — 1 of its 7 gating targets is buildable today, 2 need scoped reads, 4
are blocked on the architecture-metrics family (#156). The T3 spec folds the
tracker/target requirements in
[`design/integrations-and-gating-requirements.md`](../../../design/integrations-and-gating-requirements.md).

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