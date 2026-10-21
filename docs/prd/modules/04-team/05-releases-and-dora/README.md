# Releases, DORA & Release Gates · T5

**Plane:** team · **Status:** not built — [roadmap/09-dojo-expansion.md](../../../../roadmap/09-dojo-expansion.md) Wave D ·
**Serves:** manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
org admin

## Purpose

A workspace's **release pipelines** are read into the dōjō, so every deployment is a
record: what shipped, from which commit, to which environment, and whether it
held. From those records the dōjō computes the five **DORA** keys and holds each
release to a **release gate** — the T3 code targets, evaluated over everything the
release carries, plus the checks that say the release is safe to ship.

> **A release ships only what passed; the five keys say how the delivery is going.**

The user feels this as *I can see how often we ship, how long a change takes to
reach production, how often it breaks and how fast we recover — and a release that
would ship a regression stops and says why.*

## Decisions this module rests on (2026-10-21)

- **External systems are read-only; the dōjō pulls.** CI and tracker APIs are read,
  never written ([roadmap/09 D2–D3](../../../../roadmap/09-dojo-expansion.md#decisions-user-2026-10-21)).
- **Signals:** deployments from **GitHub Actions** and **Azure Pipelines** through the
  workspace's integrations, with **git tags** as the fallback (D4).
- **Incidents:** a **production incident** is its own entity, separate from the
  confidentiality incidents T2's lead console records (`dojo.incidents`).
- **Sync:** sensei ⇄ dōjō is two-way — release and gate state appear in sensei; the
  dōjō is the authority.

## In scope / out of scope

- In: **pipeline connections** (on T3's per-workspace integrations: GitHub Actions,
  Azure Pipelines; git tags as fallback); **deployment records** (workspace, repo,
  commit, environment, started/finished, outcome); **production incidents** (opened,
  restored, severity, linked deployment); the **five DORA keys** per workspace and per
  team, with trailing windows; **release gates** (the T3 targets over the release's
  change set + release-readiness checks + the stability guardrail); a **release
  scorecard**; the **DORA view** in the dōjō and in sensei.
- Out: **deploying anything** — sensei never triggers, approves or blocks a pipeline
  run inside the CI system (read-only); **per-author DORA** (never); incident
  management (paging, on-call) — incidents are read from a tracker label or recorded
  by hand; the fifth "generative culture" key (a survey, not a signal).

## Surfaces

- **Dōjō** — Workspace › **Releases** (drawn in the mockup, `dojo2-project.jsx`): the five keys, the three-part gate, deployments from main, and production incidents with "Record incident"; a **Releases** tab (deployments, the gate result of each,
  the scorecard) and a **Delivery** card with the five keys; the org **DORA** view
  (per workspace, per team, trailing 30/90 days); a **production incidents** list.
- **Sensei** — the workspace window shows the latest release, its gate result and the
  five keys (pulled from the dōjō); a release that is blocked names the failing
  target.
- **Data (proposed, dōjō)** — `dojo.pipeline_connections` (on the C1 connection model),
  `dojo.deployments`, `dojo.production_incidents`, `dojo.release_gates` (resolved
  result per release); the keys as rows in the existing metric plumbing
  (`repository_metrics`, `metric_activations`).

## The five keys

All five are computed per **workspace** (and rolled up per **team**), over a trailing
window, from deployments to the **production** environment.

| Key | Definition | Signal |
|---|---|---|
| Deployment frequency | successful production deployments per day/week | `deployments` where environment = production and outcome = success |
| Lead time for changes | median time from a commit's first appearance to its first successful production deployment | commit timestamps (git) × `deployments.commit` ancestry |
| Change failure rate | share of production deployments that caused a production incident or were rolled back | `production_incidents.deployment_id`, rollback deployments |
| Time to restore | median time from a production incident opening to its restore | `production_incidents.opened_at → restored_at` |
| Rework rate | share of production deployments that were unplanned fixes for a user-facing defect (DORA 2024) | a deployment whose change set closes a tracker ticket of type Bug, or that ships from a hotfix branch or tag — the workspace's convention, set on its integrations; with neither signal, "not measured" |

Five keys, not the classic four (user, 2026-10-21): rework rate is DORA's 2024 addition
and sits with change failure rate on the stability side. Each key shows the number of
deployments it is computed from, and "not measured" is never a zero.

**No benchmark bands (V5).** The keys carry no elite / high / medium / low bands. Change
failure rate, time to restore and rework rate read **over / within the workspace's
stability-guardrail threshold** (below), which the viewer (T6) also shows; deployment
frequency and lead time carry no threshold.

**Low sample (V6).** A key computed from **fewer than 5 deployments** in the window is
flagged *low sample* — `0% · 0 of 2 deploys · low sample` — never shown as a confident value.

**Group size.** A key is shown however few people contributed — an aggregate over one
contributor is still an aggregate, never a per-person view (user, 2026-10-21). This is
revisited only if comparison charts across people or teams are added.

## Release gates

A **release** is a production deployment candidate: the change set between the last
successful production deployment and the candidate commit. A release passes its gate in
each **environment in order — development → testing → production** (D18): a break in
testing predicts one in production. Production deployments come from the production branch
(the forge's default branch unless mapped); sprint targets read development — M4's branch
scope. Its gate has three parts,
all evaluated **before** the release is recorded as shippable in the dōjō and shown
in sensei:

1. **Code targets over the release's change set.** T3's seven targets, inheritance
   ladder and modes (`block` / `warn` / `off`), evaluated over every change the
   release carries rather than one sprint's. The ladder gains a **Release** scope
   (Dōjō → Workspace → **Release** → Sprint → Task type → Task).
2. **Release readiness.** The pipeline run for the candidate is green; no blocking
   **production incident** is open on the workspace; a release that misses a `block`
   target cannot be marked shippable without a note (maintainer and up).
3. **Stability guardrail (proposed default: `warn`).** When the workspace's trailing
   change failure rate, time to restore or rework rate is over its threshold, the
   release gate says so. `warn` surfaces it; `block` is opt-in per
   workspace.

## Data invariants

- **Lead.** **External systems are read-only.** A pipeline adapter reads runs and
  deployments; sensei never triggers, cancels or approves a CI run.
- **Lead.** **A gate reads the release's change set** — delta-scoped as in T3 — never
  an absolute workspace-level ratio.
- **Lead.** **The five keys are delivery insights, never per-author.** They roll up by
  workspace and team only.
- **Lead.** **Only the stability guardrail turns a key into a gate**, and only when a
  workspace opts it into `block`.
- **Lead.** **A production incident is not a confidentiality incident.** The two are
  separate entities with separate screens.
- **Lead.** **No signal is fabricated.** A workspace with no pipeline connection and no
  release tags shows "not measured", never a zero frequency or an empty gate as a pass.
- **Lead.** The **dōjō is the authority** for release and gate state; sensei reads it.

## Done gate

- A workspace with a GitHub Actions or Azure Pipelines connection lists its production
  deployments, and the five keys match a hand count over the same window.
- A workspace with only git tags lists releases from the tags and says the keys are
  tag-derived.
- A release whose change set adds a hard-rule violation is blocked and names the
  target; with a maintainer's note it can be marked shippable, and the note is kept.
- Opening and restoring a production incident linked to a deployment moves the change
  failure rate and time to restore.
- With no connection and no tags, the Releases tab says "not measured".

## Wrong gate

- A key computed per author, or shown on a person.
- A gate that reads the whole workspace's absolute ratio instead of the release's
  change set.
- Sensei writing to a CI system or tracker.
- A confidentiality incident counted toward change failure rate or time to restore.
- An empty or unconnected workspace reading as "0 deployments" or "gate passed".

## Related

T3 (targets, ladder, modes — [`../03-dojo-workspace-planning/README.md`](../03-dojo-workspace-planning/README.md)) ·
M8 metrics plumbing ([`../../02-insight/08-metrics-and-health/README.md`](../../02-insight/08-metrics-and-health/README.md)) ·
[`design/integrations-and-gating-requirements.md`](../../../../design/integrations-and-gating-requirements.md) ·
[`spec/dojo/governance/default-constitution.md`](../../../../spec/dojo/governance/default-constitution.md) (the DORA signals) ·
[`roadmap/09-dojo-expansion.md`](../../../../roadmap/09-dojo-expansion.md).
