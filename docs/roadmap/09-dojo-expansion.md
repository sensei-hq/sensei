# Dōjō expansion: workspaces, teams, integrations, two-way sync, DORA

Written 2026-10-21 from a gap inventory of `docs/prd/`, `docs/mockups/` and the code. The user's
goal: a user can create and change workspaces and teams, assign roles and connect integrations
(Jira, Azure DevOps, GitHub, Slack) — in the dōjō first or in sensei first, synced both ways — and
track release pipelines and DORA metrics. The dōjō is nearly empty today; this is the bump.

## Decisions (user, 2026-10-21)

| # | Question | Decision |
|---|---|---|
| D1 | Which role ladder is canonical? | **The database enum**: `contributor < maintainer < lead < admin` — what RLS enforces. The PRD (`member < contributor < lead < maintainer < admin`) and the mockup (`developer < …`) are updated to match. Workspace-scoped roles raise within this ladder. |
| D2 | Where do integrations live? | **The dōjō owns them.** Connected once per workspace in the dōjō, credentials held by the dōjō, shared by the team. The daemon pulls what they mirror through sync; connecting from sensei forwards to the dōjō. Supersedes the daemon-local `sensei.trackers` proposal in `docs/design/integrations-and-gating-requirements.md` §5. |
| D3 | Does "both ways" include writing to trackers (Jira, Azure Boards, GitHub Issues — the work-item systems whose tickets and sprints a session links to)? | **No — dōjō ⇄ sensei only.** Workspaces, teams, roles and connections sync both ways between the dōjō and sensei; Jira / ADO / GitHub stay read-only (the 2026-10-14 decision stands). |
| D4 | Where do DORA signals come from? | **CI APIs plus git tags.** Deployments from GitHub Actions and Azure Pipelines through the integrations, git tags as the fallback; a production-incident entity, separate from today's confidentiality `dojo.incidents`, for time to restore and change failure rate. |
| D5 | Where do repository permissions come from? (2026-10-21, second round) | **The forge, with an admin override.** A person's role on a repository mirrors the git provider's per-repo access (GitHub / Azure DevOps / GitLab), refreshed on sync; a dōjō admin may override it. **Permissions live on repositories** — a workspace is a grouping of repositories and its own permission is only the read-only view (D8). Supersedes the 2026-10-12 decision that team membership grants access. |
| D6 | Who sees per-person data for a repository? | **The person, and maintainers and above on that repository.** Workspace viewers see aggregates and per-repository splits, never per-person. |
| D7 | What are teams for? | **Groupings, not access** — named groups of people for "by team" reports and for granting workspace-viewer access in bulk. Never a source of repository access, so no default team owns workspaces. |
| D8 | What is a workspace role? | **A read-only managerial viewer**: metrics, dashboards and reports for the workspace, aggregated or split by repository — for higher management and clients. **Admins grant it**, and only to someone already in the dōjō: a member, or a **dōjō viewer** — a person (often in the organization but not a contributor, e.g. a manager or a client) explicitly invited into the dōjō as a viewer, which is not a membership and reads no repository. The dōjō invite comes first; the per-workspace grant second (revised 2026-10-21). Not an access-management role. |
| D9 | Four DORA keys or five? | **Five** — rework rate (DORA 2024) joins deployment frequency, lead time, change failure rate and time to restore. |
| D10 | Hide aggregates over very small groups? | **No.** An aggregate over one contributor is shown; revisit only if comparison charts across people or teams are added. |
| D11 | What kinds of dōjō are there? (2026-10-21, third round) | **Two: an organization's, or a personal one.** A dōjō is the tenant — the owner. There is no hand-made dōjō: `dojo.create_dojo`, the `team` tenant origin, `team/<slug>` keys and the `team:<slug>` pack namespace are retired. |
| D12 | What is a "workspace"? | **A workspace** — a group of repositories. Renamed everywhere, in one slice: the database, the API, the daemon's sync, both apps and the docs (sensei and the dōjō are both pre-v1). "Workspace" stops being a term. |
| D13 | What does linking a team to a workspace do? | **Reporting only.** "Teams work on workspaces": the link says which team works on which workspace, so reports split by team. It grants nothing — access stays on repositories (D5) and viewer grants (D8). |

**Viewer decisions V1–V12 (2026-10-21)** — recorded in
[T6 Workspace Viewer](../prd/modules/04-team/06-workspace-viewer/README.md#decisions-taken-2026-10-21): the
quality headline against a goal the workspace sets (V2); the ratchet advances on the trailing
3-sprint median (V3) for coverage, zone of pain and changed-function complexity (V4); no DORA
benchmark bands, stability keys against the workspace's guardrail (V5); *low sample* under 5
deploys (V6); one-person repositories shown (V7); admins grant (V8); no export for outsiders
(V9); generated health text plus a maintainer note (V10); tracker milestones (V11); a partial
index is not evaluated (V12).

Already decided elsewhere and kept: **the dōjō wins on conflict** (`docs/spec/2026-08-24-platform-restructure.md` Q1);
an `origin` column stops push/pull loops (`docs/spec/2026-08-24-shared-schema-and-sync-design.md`).
Workspaces may now be **created in the dōjō**, which reverses the invariant that the sync is their only
creator (`database/ddl/table/dojo/projects.ddl`); the sync keeps filing repositories into them.

## Where it stands (inventory, 2026-10-21)

| Area | Exists |
|---|---|
| Workspaces | Read-only lists (`/v1/t/…/projects`, `/v1/you/projects`); created only by the sync (`file_repository_project`). No create / edit / move-repo; the org workspace drill-in reads fixtures. |
| Teams & roles | *Since then: teams, invites and viewers are built (A3); create-a-dōjō is retired (R1).* At the inventory: member role change, create-a-dōjō, invite accept and GitHub provisioning work. `dojo.teams`, `team_members`, `team_projects`, `roles` exist but nothing writes them; no default team is seeded; no Teams screen or API; the invite API has no screen. |
| Integrations | GitHub org identity only (`tenant_connections`, `forge-github.ts`). No tracker or CI adapter, table, route or screen. |
| Two-way sync | Daemon → dōjō: repositories, metrics, graph, relay. Dōjō → daemon: sync plan, memberships, resolved rules, relay replies. No entity is authored on one side and mirrored to the other. |
| Pipelines & DORA | Nothing. The metric plumbing (`repository_metrics`, `metric_activations`, `/v1/you/metrics`) is reusable. |

## Waves

Each wave depends only on earlier ones. Every slice ships with tests, is verified live (the local
dōjō, then production), and keeps presentation | state | load apart with rokkit first (#293).

### Wave R — one vocabulary (D11, D12), before A4

D12 renames "workspace" to **workspace** everywhere, and D11 retires the hand-made dōjō. Both land before
A4, so the writable workspaces are built under their real name. Sized 2026-10-21: "workspace" appears in
~290 crate files (~7.3k lines), ~260 app files (~2.5k), ~160 dōjō files (~1.3k), ~490 docs (~6.4k);
`sensei.projects` and ~30 related tables, views and functions; `dojo.projects` and its links. Each
slice builds, passes and ships on its own.

- **R1 · Retire the hand-made dōjō (D11).** *Done (7478fe8d).* Drop `dojo.create_dojo`, the `team` tenant origin, the
  `team:<slug>` pack namespace and the "create a dōjō" path in the dōjō UI and API; a dōjō comes only
  from an organization or a person.
- **R2 · The dōjō says workspace.** `dojo.projects` → `dojo.workspaces` with its links, viewer grants,
  team links (reporting only, D13), predicates, policies and tests; the `/v1` paths and payloads; the
  dōjō UI. The daemon's dōjō client and relay payload change in the same release (they share the wire).
- **R3 · Sensei says workspace.** `sensei.projects` and its tables, views and functions; the daemon's
  routes and pipeline; the MCP tools (`list_projects`, `use_project`, `get_project_summary`, …) and the
  plugin, skills and hooks that call them; the desktop app.
- **R4 · The words.** PRD, specs, guides, `docs/llms`, both `SKILL.md` copies, agents, the website.
  Generic English ("the workspace's README", a Cargo workspace) stays.

### Wave A — the dōjō can author workspaces, teams and roles

- **A1 · One role ladder.** The PRD and docs follow D1 (5564841d). ~~Workspace-scoped acting roles~~
  (a053772d) are reshaped by D8 into the **workspace viewer** grant — read-only, admin-granted, open to
  viewer-only guests.
- **A2 · Repository roles (D5, D6).** *Done (A2a–A2d: 3a0712d9, 8de8639a, 20248297, 6aa2d690; A2d's live check waits on a daemon install).* `repository_roles` per person per repository: mirrored from the
  forge's per-repo access on sync (read / triage / write → contributor, maintain → maintainer, admin →
  admin; `lead` stays the tenant's confidentiality role), an admin override that survives the next
  mirror. The access predicates move from team membership to repository roles: a repository's shared
  rows for anyone with a role on it, per-person rows for the person and maintainers and above.
- **A3 · Teams as groupings (D7).** *Done: teams (f01422ee, d200c4d1, 2e42d46d), the invite screen with viewer invites (18a532af, 7e547a19), viewers (2830d9f6, 734cb6ca). The `team/<slug>` vocabulary item became R1 (D11).* Team CRUD and membership; reports "by team"; grant workspace viewer
  to a whole team. The self-made dōjō's `team/<slug>` tenant vocabulary renamed so "team" means one
  thing. An invite screen over the existing invite API, including viewer-only guests (D8).
- **A4 · Workspaces are writable.** Create, rename and archive a workspace; add, move and remove a
  repository (admins and maintainers of the repositories involved). The workspace detail page with its
  Repos and People (viewers, per [T6](../prd/modules/04-team/06-workspace-viewer/README.md)) tabs; the org drill-in off fixtures.

### Wave B — dōjō ⇄ sensei, both ways

- **B1 · Dōjō → sensei.** The daemon pulls workspaces, their repositories, teams and roles each cycle; a
  workspace made in the dōjō appears in sensei. `origin` marks where a row was authored; the dōjō wins.
- **B2 · Sensei → dōjō.** Creating or changing a workspace or team in sensei writes to the dōjō (through
  the daemon), then arrives back through B1 — one authority, no second copy.
- **B3 · Conflicts are visible.** When a local edit loses to the dōjō, sensei says so rather than
  silently reverting.

### Wave C — integrations, owned by the dōjō, read-only

- **C1 · Connections.** A per-workspace connection model (provider, scope, status
  `connected / stale / error`), OAuth with a PAT fallback, credentials held by the dōjō.
- **C2 · Trackers.** GitHub Issues, Jira and Azure Boards adapters mirroring tickets and sprints; the
  key pattern that links a session to a ticket (branch, commit, prompt); the Integrations tab.
- **C3 · Sensei reads them.** The daemon pulls the mirrored tickets so sessions link to tickets locally.

### Wave D — release pipelines, DORA and release gates

Specified in the PRD as [T5](../prd/modules/04-team/05-releases-and-dora/README.md).

- **D-1 · Deployments.** GitHub Actions and Azure Pipelines adapters (on the C1 connections) record
  deployments; git tags are the fallback.
- **D-2 · Production incidents.** An entity separate from confidentiality incidents: opened, restored,
  linked to the deployment that caused it.
- **D-3 · The five keys.** Deployment frequency, lead time for changes, change failure rate, time to
  restore and rework rate (D9) as metrics in the existing plumbing; a DORA view in the dōjō and in sensei.
- **D-4 · Release gates.** T3's targets, ladder (gaining a **Release** scope) and modes
  evaluated over a release's change set; release readiness (green pipeline, no open
  blocking production incident); the stability guardrail (change failure rate / time to
  restore over threshold — `warn` by default, `block` opt-in); the release scorecard.
  Specified in [T5](../prd/modules/04-team/05-releases-and-dora/README.md).

## Order against the burndown

Bugs first: #305 (inbox answers to a gate are never sent) and the fold of `/you/runs/[run_id]` into
`RelayDetail` finish #293 step 1 — *both done (bf16a437, 3d443b1b)*. Then Wave A. The rest of the burndown (#293 step 2 rokkit, #294,
#297, #298 …) interleaves where a wave touches the same screen.
