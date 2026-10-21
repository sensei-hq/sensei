# Releases, DORA & Release Gates · T5

**Plane:** team · **Status:** not built — [roadmap/09-dojo-expansion.md](../../../../roadmap/09-dojo-expansion.md) Wave D ·
**Serves:** manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
org admin

## Purpose

A project's **release pipelines** are read into the dōjō, so every deployment is a
record: what shipped, from which commit, to which environment, and whether it
held. From those records the dōjō computes the four **DORA** keys and holds each
release to a **release gate** — the T3 code targets, evaluated over everything the
release carries, plus the checks that say the release is safe to ship.

> **A release ships only what passed; the four keys say how the delivery is going.**

The user feels this as *I can see how often we ship, how long a change takes to
reach production, how often it breaks and how fast we recover — and a release that
would ship a regression stops and says why.*

## Decisions this module rests on (2026-10-21)

- **External systems are read-only; the dōjō pulls.** CI and tracker APIs are read,
  never written ([roadmap/09 D2–D3](../../../../roadmap/09-dojo-expansion.md#decisions-user-2026-10-21)).
- **Signals:** deployments from **GitHub Actions** and **Azure Pipelines** through the
  project's integrations, with **git tags** as the fallback (D4).
- **Incidents:** a **production incident** is its own entity, separate from the
  confidentiality incidents T2's lead console records (`dojo.incidents`).
- **Sync:** sensei ⇄ dōjō is two-way — release and gate state appear in sensei; the
  dōjō is the authority.

## In scope / out of scope

- In: **pipeline connections** (on T3's per-project integrations: GitHub Actions,
  Azure Pipelines; git tags as fallback); **deployment records** (project, repo,
  commit, environment, started/finished, outcome); **production incidents** (opened,
  restored, severity, linked deployment); the **four DORA keys** per project and per
  team, with trailing windows; **release gates** (the T3 targets over the release's
  change set + release-readiness checks + the stability guardrail); a **release
  scorecard**; the **DORA view** in the dōjō and in sensei.
- Out: **deploying anything** — sensei never triggers, approves or blocks a pipeline
  run inside the CI system (read-only); **per-author DORA** (never); incident
  management (paging, on-call) — incidents are read from a tracker label or recorded
  by hand; the fifth "generative culture" key (a survey, not a signal).

## Surfaces

- **Dōjō** — project detail: a **Releases** tab (deployments, the gate result of each,
  the scorecard) and a **Delivery** card with the four keys; the org **DORA** view
  (per project, per team, trailing 30/90 days); a **production incidents** list.
- **Sensei** — the project window shows the latest release, its gate result and the
  four keys (pulled from the dōjō); a release that is blocked names the failing
  target.
- **Data (proposed, dōjō)** — `dojo.pipeline_connections` (on the C1 connection model),
  `dojo.deployments`, `dojo.production_incidents`, `dojo.release_gates` (resolved
  result per release); the keys as rows in the existing metric plumbing
  (`repository_metrics`, `metric_activations`).

## The four keys

All four are computed per **project** (and rolled up per **team**), over a trailing
window, from deployments to the **production** environment.

| Key | Definition | Signal |
|---|---|---|
| Deployment frequency | successful production deployments per day/week | `deployments` where environment = production and outcome = success |
| Lead time for changes | median time from a commit's first appearance to its first successful production deployment | commit timestamps (git) × `deployments.commit` ancestry |
| Change failure rate | share of production deployments that caused a production incident or were rolled back | `production_incidents.deployment_id`, rollback deployments |
| Time to restore | median time from a production incident opening to its restore | `production_incidents.opened_at → restored_at` |

## Release gates

A **release** is a production deployment candidate: the change set between the last
successful production deployment and the candidate commit. Its gate has three parts,
all evaluated **before** the release is recorded as shippable in the dōjō and shown
in sensei:

1. **Code targets over the release's change set.** T3's seven targets, inheritance
   ladder and modes (`block` / `warn` / `off`), evaluated over every change the
   release carries rather than one sprint's. The ladder gains a **Release** scope
   (Dōjō → Project → **Release** → Sprint → Task type → Task).
2. **Release readiness.** The pipeline run for the candidate is green; no blocking
   **production incident** is open on the project; a release that misses a `block`
   target cannot be marked shippable without a note (maintainer and up).
3. **Stability guardrail (proposed default: `warn`).** When the project's trailing
   change failure rate is over its threshold, or time to restore is over its
   threshold, the release gate says so. `warn` surfaces it; `block` is opt-in per
   project.

## Data invariants

- **Lead.** **External systems are read-only.** A pipeline adapter reads runs and
  deployments; sensei never triggers, cancels or approves a CI run.
- **Lead.** **A gate reads the release's change set** — delta-scoped as in T3 — never
  an absolute project-level ratio.
- **Lead.** **The four keys are delivery insights, never per-author.** They roll up by
  project and team only.
- **Lead.** **Only the stability guardrail turns a key into a gate**, and only when a
  project opts it into `block`.
- **Lead.** **A production incident is not a confidentiality incident.** The two are
  separate entities with separate screens.
- **Lead.** **No signal is fabricated.** A project with no pipeline connection and no
  release tags shows "not measured", never a zero frequency or an empty gate as a pass.
- **Lead.** The **dōjō is the authority** for release and gate state; sensei reads it.

## Done gate

- A project with a GitHub Actions or Azure Pipelines connection lists its production
  deployments, and the four keys match a hand count over the same window.
- A project with only git tags lists releases from the tags and says the keys are
  tag-derived.
- A release whose change set adds a hard-rule violation is blocked and names the
  target; with a maintainer's note it can be marked shippable, and the note is kept.
- Opening and restoring a production incident linked to a deployment moves the change
  failure rate and time to restore.
- With no connection and no tags, the Releases tab says "not measured".

## Wrong gate

- A key computed per author, or shown on a person.
- A gate that reads the whole project's absolute ratio instead of the release's
  change set.
- Sensei writing to a CI system or tracker.
- A confidentiality incident counted toward change failure rate or time to restore.
- An empty or unconnected project reading as "0 deployments" or "gate passed".

## Related

T3 (targets, ladder, modes — [`../03-dojo-project-planning/README.md`](../03-dojo-project-planning/README.md)) ·
M8 metrics plumbing ([`../../02-insight/08-metrics-and-health/README.md`](../../02-insight/08-metrics-and-health/README.md)) ·
[`design/integrations-and-gating-requirements.md`](../../../../design/integrations-and-gating-requirements.md) ·
[`spec/dojo/governance/default-constitution.md`](../../../../spec/dojo/governance/default-constitution.md) (the DORA signals) ·
[`roadmap/09-dojo-expansion.md`](../../../../roadmap/09-dojo-expansion.md).
