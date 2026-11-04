# Team plane — the Dōjō

> Team = T1–T6. The cross-cutting team layer: knowledge travels up, is governed,
> and comes back down — the org boundary is the Dōjō membership. Status:
> [`../../../roadmap/04-team.md`](../../../roadmap/04-team.md).

| ID | Module | One-line "done" | Spec |
|---|---|---|---|
| T1 | Dōjō Core | Knowledge travels up, is governed, and comes back down | [`README`](01-dojo-core/README.md) · [`dojo-core`](01-dojo-core/scenarios/dojo-core.feature) · [`dojo-core-gaps`](01-dojo-core/scenarios/dojo-core-gaps.feature) |
| T2 | Dōjō Consoles | All four roles (contributor · maintainer · lead · admin) have a live console | [`README`](02-dojo-consoles/README.md) · [`dojo-consoles`](02-dojo-consoles/scenarios/dojo-consoles.feature) · [`dojo-consoles-gaps`](02-dojo-consoles/scenarios/dojo-consoles-gaps.feature) |
| T3 | Dōjō Workspace Planning | A workspace plans against its tracker and sets code-health targets; a blocking target gates work | [`README`](03-dojo-workspace-planning/README.md) · [`dojo-workspace-planning`](03-dojo-workspace-planning/scenarios/dojo-workspace-planning.feature) · [`dojo-workspace-planning-gaps`](03-dojo-workspace-planning/scenarios/dojo-workspace-planning-gaps.feature) |
| T4 | Billing & Seats | An organisation's private repositories take the paid tier — targets, integrations, consolidated metrics — and are metered; solo, personal and public are free | [`README`](04-billing-seats/README.md) · [`billing-seats`](04-billing-seats/scenarios/billing-seats.feature) |
| T5 | Releases, DORA & Release Gates | Deployments are read from CI, the five DORA keys are computed, and a release that would ship a regression is held | [`README`](05-releases-and-dora/README.md) · [`releases-and-dora`](05-releases-and-dora/scenarios/releases-and-dora.feature) |
| T6 | Workspace Viewer | A manager or client sees a granted workspace's outcomes — never who did what, never the code | [`README`](06-workspace-viewer/README.md) · [`workspace-viewer`](06-workspace-viewer/scenarios/workspace-viewer.feature) |

## What this plane owns

Memberships, routing, preview and the universal strip (T1); the role consoles
(T2 — the four Dōjō roles live in [`02-objectives.md`](../../02-objectives.md) §Dōjō:
contributor < maintainer < **lead** < admin (D1); the lead is the team lead (D38, D58);
a member manager is a maintainer or lead, and an outside manager or client is a
**viewer** — not a role, aggregates only (D8, D54));
workspace planning against external trackers (Jira / ADO) with
**delta-scoped, ratchet-fair quality targets** (T3, reorg §9; D47); the free-where-public
business model — open source; a solo developer gets everything free, and the paid tier is an
organisation's private repositories: targets, integrations and consolidated metrics (T4, D52); and release
pipelines, the five DORA keys and **release gates** that hold a release to the T3
targets over its change set (T5).

**Resolved** ([roadmap/09](../../../roadmap/09-dojo-expansion.md)): external
systems (trackers, CI) are read; **ticket writeback is opt-in per workspace, off by default**
(D50, amending D3). **Org workspaces use dōjō-owned tracker connections** (D49; needs the
secret store, #310), and new adapters (GitHub Issues, ADO) go on that path; **the daemon path
stays for solo and personal use** (D22, D14, D21). **sensei ⇄ dōjō is two-way sync** for workspaces, teams,
roles and connections, with the dōjō the authority on conflict. The canonical role
ladder is `contributor < maintainer < lead < admin`.

## Reading order

T1 → T2 → T3 → T4 → T5 → T6 (T5 builds on T3's targets and integrations; T6, the viewer, reads T3 and T5). T3 is the newest surface; read [`../../../reorg.md`](../../../reorg.md) §9
first — 1 of its 7 gating targets is buildable today, 2 need scoped reads, 4
are blocked on the architecture-metrics family (#156). The T3 spec folds the
tracker/target requirements in
[`design/integrations-and-gating-requirements.md`](../../../design/integrations-and-gating-requirements.md).

## Customer

The Team plane serves every persona this pass added — **manager / delivery-lead**
and **client / workspace stakeholder** — as well as the maintainer, admin and lead
roles. It is the payload for the platform's billing question.

## Legacy sources

Folded from `features/05-governance.md`, `features/07-pricing.md`,
`features/dojo/`, `spec/screen/dojo-*.md`,
`spec/pipeline/{dojo-lifecycle,governance}`,
`spec/dojo/{dojo-auth-provisioning.md,…}`,
`architecture/{dojo,dojo-deployment,entity-access-model}`,
`journeys/dojo.md`, `mockups/Sensei/lib/dojo2/` (visual source — never edited).