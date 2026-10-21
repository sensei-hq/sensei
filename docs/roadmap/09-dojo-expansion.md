# Dōjō expansion: projects, teams, integrations, two-way sync, DORA

Written 2026-10-21 from a gap inventory of `docs/prd/`, `docs/mockups/` and the code. The user's
goal: a user can create and change projects and teams, assign roles and connect integrations
(Jira, Azure DevOps, GitHub, Slack) — in the dōjō first or in sensei first, synced both ways — and
track release pipelines and DORA metrics. The dōjō is nearly empty today; this is the bump.

## Decisions (user, 2026-10-21)

| # | Question | Decision |
|---|---|---|
| D1 | Which role ladder is canonical? | **The database enum**: `contributor < maintainer < lead < admin` — what RLS enforces. The PRD (`member < contributor < lead < maintainer < admin`) and the mockup (`developer < …`) are updated to match. Project-scoped roles raise within this ladder. |
| D2 | Where do integrations live? | **The dōjō owns them.** Connected once per project in the dōjō, credentials held by the dōjō, shared by the team. The daemon pulls what they mirror through sync; connecting from sensei forwards to the dōjō. Supersedes the daemon-local `sensei.trackers` proposal in `docs/design/integrations-and-gating-requirements.md` §5. |
| D3 | Does "both ways" include writing to trackers (Jira, Azure Boards, GitHub Issues — the work-item systems whose tickets and sprints a session links to)? | **No — dōjō ⇄ sensei only.** Projects, teams, roles and connections sync both ways between the dōjō and sensei; Jira / ADO / GitHub stay read-only (the 2026-10-14 decision stands). |
| D4 | Where do DORA signals come from? | **CI APIs plus git tags.** Deployments from GitHub Actions and Azure Pipelines through the integrations, git tags as the fallback; a production-incident entity, separate from today's confidentiality `dojo.incidents`, for time to restore and change failure rate. |

Already decided elsewhere and kept: **the dōjō wins on conflict** (`docs/spec/2026-08-24-platform-restructure.md` Q1);
an `origin` column stops push/pull loops (`docs/spec/2026-08-24-shared-schema-and-sync-design.md`).
Projects may now be **created in the dōjō**, which reverses the invariant that the sync is their only
creator (`database/ddl/table/dojo/projects.ddl`); the sync keeps filing repositories into them.

## Where it stands (inventory, 2026-10-21)

| Area | Exists |
|---|---|
| Projects | Read-only lists (`/v1/t/…/projects`, `/v1/you/projects`); created only by the sync (`file_repository_project`). No create / edit / move-repo; the org project drill-in reads fixtures. |
| Teams & roles | Member role change, create-a-dōjō, invite accept and GitHub provisioning work. `dojo.teams`, `team_members`, `team_projects`, `roles` exist but nothing writes them; no default team is seeded; no Teams screen or API; the invite API has no screen. |
| Integrations | GitHub org identity only (`tenant_connections`, `forge-github.ts`). No tracker or CI adapter, table, route or screen. |
| Two-way sync | Daemon → dōjō: repositories, metrics, graph, relay. Dōjō → daemon: sync plan, memberships, resolved rules, relay replies. No entity is authored on one side and mirrored to the other. |
| Pipelines & DORA | Nothing. The metric plumbing (`repository_metrics`, `metric_activations`, `/v1/you/metrics`) is reusable. |

## Waves

Each wave depends only on earlier ones. Every slice ships with tests, is verified live (the local
dōjō, then production), and keeps presentation | state | load apart with rokkit first (#293).

### Wave A — the dōjō can author projects, teams and roles

- **A1 · One role ladder.** Update the PRD and the mockup vocabulary to D1. Add project-scoped role
  overrides (raise only, within the ladder) with RLS.
- **A2 · Teams are real.** Seed a default team per tenant (provisioning and `create_dojo`); team CRUD
  and membership; assign teams to projects (`team_projects`); a Teams screen; an invite screen over
  the existing invite API. Rename the self-made dōjō's `team/<slug>` tenant key vocabulary so "team"
  means one thing.
- **A3 · Projects are writable.** Create, rename and archive a project; add, move and remove a
  repository (security-definer functions, maintainer and up). The project detail page with its Repos
  and Team tabs; the org drill-in off fixtures.

### Wave B — dōjō ⇄ sensei, both ways

- **B1 · Dōjō → sensei.** The daemon pulls projects, their repositories, teams and roles each cycle; a
  project made in the dōjō appears in sensei. `origin` marks where a row was authored; the dōjō wins.
- **B2 · Sensei → dōjō.** Creating or changing a project or team in sensei writes to the dōjō (through
  the daemon), then arrives back through B1 — one authority, no second copy.
- **B3 · Conflicts are visible.** When a local edit loses to the dōjō, sensei says so rather than
  silently reverting.

### Wave C — integrations, owned by the dōjō, read-only

- **C1 · Connections.** A per-project connection model (provider, scope, status
  `connected / stale / error`), OAuth with a PAT fallback, credentials held by the dōjō.
- **C2 · Trackers.** GitHub Issues, Jira and Azure Boards adapters mirroring tickets and sprints; the
  key pattern that links a session to a ticket (branch, commit, prompt); the Integrations tab.
- **C3 · Sensei reads them.** The daemon pulls the mirrored tickets so sessions link to tickets locally.

### Wave D — release pipelines and DORA

- **D-1 · Deployments.** GitHub Actions and Azure Pipelines adapters (on the C1 connections) record
  deployments; git tags are the fallback.
- **D-2 · Production incidents.** An entity separate from confidentiality incidents: opened, restored,
  linked to the deployment that caused it.
- **D-3 · The four keys.** Deployment frequency, lead time for changes, change failure rate and time to
  restore as metrics in the existing plumbing; a DORA view in the dōjō and in sensei.

## Order against the burndown

Bugs first: #305 (inbox answers to a gate are never sent) and the fold of `/you/runs/[run_id]` into
`RelayDetail` finish #293 step 1. Then Wave A. The rest of the burndown (#293 step 2 rokkit, #294,
#297, #298 …) interleaves where a wave touches the same screen.
