# Workspace Viewer · T6

**Plane:** team · **Status:** partly built — the grant model and its admin surface are live
(#307 A3); the viewer shell is mockup only (`docs/mockups/Sensei/lib/dojo2/dojo2-viewer.jsx`) ·
**Serves:** manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up)),
client / workspace stakeholder ([persona 7](../../../03-personas.md#7--client--workspace-stakeholder--the-narrow-lens)),
org admin ([persona 4](../../../03-personas.md#4--org-admin--the-plane-owner)) as the one who grants

## Purpose

A read-only window onto one workspace's **outcomes** for people who don't work in it: higher
management inside the organization and client stakeholders outside it. They see delivery (the
T5 keys and release gates) and code quality (the T3 targets) for the workspaces they were
granted, aggregated or split by repository, and nothing else.

> **A viewer sees how the workspace is doing, never who did what or how the code reads.**

The user feels this as *five minutes on Monday: is it shipping, is it getting better, is the
next release held and why — without asking the team for a status deck.*

## Decisions this module rests on

- **D8 (revised 2026-10-21).** Viewing is a **grant on a workspace**, not a dōjō role and not an
  access-management role. **Admins grant it**, and only to someone already in the dōjō: a
  member, or a **dōjō viewer** — a person (often a manager or a client, with no repository
  access) explicitly invited into the dōjō as a viewer, which is not a membership. The dōjō
  invite comes first; the per-workspace grant second.
- **D6.** Viewers see aggregates and per-repository splits, **never per-person**.
- **D7, D13, D15.** Teams are groupings: a grant can go to everyone on a team at once — one
  grant per person on the team at that moment — and reports can group "by team". A team
  grants nothing by itself and never owns a workspace.
- **D15.** Grants carry an expiry and an export flag; only an admin marks a viewer as outside
  the organization (the relationship enum); the viewer shell is to be built (D15).
- **D10.** Aggregates over one contributor are shown, and so is a per-repository split when one
  person works on that repository.
- **V1–V12 (2026-10-21)** — under *Decisions taken* below.

## In scope / out of scope

- In: the **viewer shell** (no dōjō switcher, no search, no needs-you bell); a **shared
  workspaces home** when the viewer holds 2+ grants and a **direct landing** on the workspace
  when they hold one; the **workspace overview** (delivery, quality headline, releases &
  incidents, repository health, milestones & scope); the **repository view**; **sprints &
  results** rolled up by task type or by team; the **viewer invite and accept**; the **admin
  grant surface** (per workspace or per team, revoke, audit); the **empty and failure states**;
  **PDF export** for internal viewers.
- Out: anything a viewer can change; per-person numbers (D6); code, diffs, file or module
  paths, branch names, ticket keys and titles; session transcripts; AI usage per person (one
  workspace-total line only, never gated); export for invited outsiders (V9).

## Surfaces

- **Dōjō · viewer shell** — `/v/{tenant}` home, `/v/{tenant}/workspaces/{id}` overview ·
  repositories · sprints, `/v/{tenant}/workspaces/{id}/repos/{repo}`. *Not built:* a viewer who
  accepts an invite lands on their workspaces list today.
- **Dōjō · admin** — Members & Roles › **Viewers** (each viewer, the workspaces granted to
  them, grant, revoke, remove) and › **Teams** (grant a workspace to everyone on a team) —
  *built* (#307 A3). The invite (member or viewer) — *built*. Workspace › Team lists the
  workspace's viewers — not built (A4).
- **Email** — invite, revoke notice. *Not built:* the invite is a link the admin shares.
- **Data (built)** — `dojo.dojo_viewers(tenant_id, principal_id, invited_by, email, joined_at)`
  (in the dōjō as a viewer, not a member) and `dojo.project_viewers(project_id,
  principal_id, granted_by, granted_at)` (the grant); reads go through `dojo.can_read_project`
  ([access matrix](../../../../spec/dojo/access-matrix.md)). A team grant writes one row per
  person on the team at the time; it outlives the team. These are the code's names today;
  R2 renames them to `workspace_viewers` / `can_read_workspace`.
- **Data (decided, D15 — to build)** — a grant gains an **expiry** (`expires_at`) and an
  **export flag** (`can_export`); a viewer's **relationship** (employer · client · community ·
  personal) is set **by an admin only** and marks who is outside the organization — someone
  outside never exports, whatever the flag says. An audit row for every grant, change, revoke,
  export and first open.

## Data invariants

- **Lead.** **Viewer reads are aggregate or per-repository.** No query a viewer can reach
  returns a row keyed to a person.
- **Lead.** **No code-level detail crosses.** Risk and health text is generated from counts by
  category; a maintainer note is plain text the maintainer wrote for this audience.
- **Lead.** **Only admins grant, change or revoke.** Every grant, change, revoke, export and
  first open is written to the audit trail.
- **Lead.** **A viewer who is not a member sees only their grants.** No member directory, no
  other workspace, no dōjō navigation, no repository.
- **Lead.** **Empty is named, never zero** — *not measured* (no source), *none in window*
  (measured, nothing happened), *not deployed* (local-only repository), *partial* (indexing).
- **Lead.** **Every rate carries its denominator** ("1 of 18 deploys").

## Signals shown

| Signal | Means | Example |
|---|---|---|
| Quality headline | share of the workspace's gated targets met this sprint, against **the workspace's own goal line** (V2) | `86% met · goal 80%` |
| Target result | met / missed, with the mode (`block` / `warn`) | `Coverage ≥ 80% — missed · blocks close` |
| Ratchet baseline | the workspace's own baseline; advances when the **trailing 3-sprint median** beats it (V3) | `coverage baseline 78% → 80% at sprint 22` |
| DORA key | a T5 key over trailing 30 / 90 days, with its count | `6% · 1 of 18 deploys` |
| Threshold | over / within the **workspace's stability-guardrail threshold** (V5); deployment frequency and lead time carry no threshold | `change failure rate 17% · over 15%` |
| Low sample | fewer than **5 deploys** in the window (V6) | `0% · 0 of 2 deploys · low sample` |
| Release gate | T5's three parts: targets over the change set · readiness · stability guardrail | `held · 1 blocking target missed` |
| Partial | a repository still indexing; its index-derived targets are **not evaluated** (V12) | `partial · 62% indexed` |
| Sources | where each number comes from and when it was pulled | `deploys from GitHub Actions · git tags as fallback · pulled 14m ago` |

## Decisions taken (2026-10-21)

| # | Question | Decision |
|---|---|---|
| V1 | Where is the viewer specified? | This module, T6. |
| V2 | Quality headline? | Share of gated targets met per sprint, against a goal line **the workspace sets** (no dōjō-wide default). |
| V3 | When does a ratchet baseline advance? | When the **trailing 3-sprint median** beats it. |
| V4 | Which targets ratchet? | Coverage on guarded paths, changed functions over complexity 10, modules in the zone of pain. The others stay delta-scoped at 0. |
| V5 | DORA bands? | No benchmark bands. Change failure rate, time to restore and rework rate show over / within the workspace's **stability-guardrail threshold** (T5). |
| V6 | Few deploys? | Shown with the count; flagged **low sample** under 5 deploys in the window. |
| V7 | One-person repository? | Shown (D10). |
| V8 | Who grants? | Admins only (D8). |
| V9 | Export for someone outside the organization? | **Never.** Internal viewers export if the grant allows it. Who is outside is an admin's call, through the relationship (D15). |
| V10 | Repository health text? | Generated from counts; a maintainer may add a note, shown with their name. |
| V11 | Milestones and scope changes? | From the tracker — Jira versions / epics, ADO milestones — through the T3 mirror. |
| V12 | A target read from a partial index? | **Not evaluated** — reads *not measured* until indexing completes. |

## Done gate

- An admin invites an outside email into the dōjō as a viewer and grants one workspace; the
  person signs in, lands on that workspace and can reach nothing else.
- A viewer with two grants lands on a home listing exactly those two.
- No viewer response carries a person id, a file path, a branch or a ticket key.
- A workspace with no CI connection shows every DORA key as *not measured*.
- A repository at 62% indexed shows coverage and complexity as *not measured* and does not
  count them in the headline.
- Revoking a grant ends the viewer's next request with "access ended", and the trail records it.
- A grant past its expiry reads as revoked.

## Wrong gate

- A viewer sees a person's name next to a number.
- An outside viewer sees an Export button, another workspace, or the dōjō switcher.
- *Not measured* renders as 0, or a 1-deploy window shows a confident 0% failure rate unflagged.
- A partial index counts toward the headline.
- A lead or maintainer can grant viewer access.
- Someone is granted a workspace without first being in the dōjō.
- Anyone but an admin can mark a viewer as inside or outside the organization.

## Open gaps

- Depends on T3 (targets, ratchet — C-1), T5 (deployments, keys, gates — Wave D) and A4 (the
  workspace detail page). Built from #307 A3: the invite, dōjō viewers, per-workspace and
  per-team grants.

## Related

D5–D13 and V1–V12 in [roadmap/09](../../../../roadmap/09-dojo-expansion.md) ·
[access matrix](../../../../spec/dojo/access-matrix.md) ·
[T3](../03-dojo-workspace-planning/README.md) · [T5](../05-releases-and-dora/README.md) ·
personas 6 and 7 · [`scenarios/workspace-viewer.feature`](scenarios/workspace-viewer.feature).
