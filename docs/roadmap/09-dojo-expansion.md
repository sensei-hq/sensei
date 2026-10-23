# Dōjō expansion: workspaces, teams, integrations, two-way sync, DORA

Written 2026-10-21 from a gap inventory of `docs/prd/`, `docs/mockups/` and the code. The user's
goal: a user can create and change workspaces and teams, assign roles and connect integrations
(Jira, Azure DevOps, GitHub, Slack) — in the dōjō first or in sensei first, synced both ways — and
track release pipelines and DORA metrics. The dōjō is nearly empty today; this is the bump.

## Decisions (user, 2026-10-21)

| # | Question | Decision |
|---|---|---|
| D1 | Which role ladder is canonical? | **The database enum**: `contributor < maintainer < lead < admin` — what RLS enforces. The PRD (`member < contributor < lead < maintainer < admin`) and the mockup (`developer < …`) are updated to match. |
| D2 | Where do integrations live? | **The dōjō owns them.** Connected once per workspace in the dōjō, credentials held by the dōjō, shared by the team. The daemon pulls what they mirror through sync; connecting from sensei forwards to the dōjō. Supersedes the daemon-local `sensei.trackers` proposal in `docs/design/integrations-and-gating-requirements.md` §5. |
| D3 | Does "both ways" include writing to trackers (Jira, Azure Boards, GitHub Issues — the work-item systems whose tickets and sprints a session links to)? | **No — dōjō ⇄ sensei only.** Workspaces, teams, roles and connections sync both ways between the dōjō and sensei; Jira / ADO / GitHub stay read-only (the 2026-10-14 decision stands). |
| D4 | Where do DORA signals come from? | **CI APIs plus git tags.** Deployments from GitHub Actions and Azure Pipelines through the integrations, git tags as the fallback; a production-incident entity, separate from today's confidentiality `dojo.incidents`, for time to restore and change failure rate. |
| D5 | Where do repository permissions come from? (2026-10-21, second round) | **The forge, with an admin override.** A person's role on a repository mirrors the git provider's per-repo access (GitHub / Azure DevOps / GitLab), refreshed on sync; a dōjō admin may override it. **Permissions live on repositories** — a workspace is a grouping of repositories and its own permission is only the read-only view (D8). Supersedes the 2026-10-12 decision that team membership grants access. |
| D6 | Who sees per-person data for a repository? | **The person, and maintainers and above on that repository.** Workspace viewers see aggregates and per-repository splits, never per-person. |
| D7 | What are teams for? | **Groupings, not access** — named groups of people for "by team" reports and for granting workspace-viewer access in bulk. Never a source of repository access, so no default team owns workspaces. |
| D8 | What is a workspace role? | **A read-only managerial viewer**: metrics, dashboards and reports for the workspace, aggregated or split by repository — for higher management and clients. **Admins grant it**, and only to someone already in the dōjō: a member, or a **dōjō viewer** — a person (often in the organization but not a contributor, e.g. a manager or a client) explicitly invited into the dōjō as a viewer, which is not a membership and reads no repository. The dōjō invite comes first; the per-workspace grant second (revised 2026-10-21). Not an access-management role. |
| D9 | Four DORA keys or five? | **Five** — rework rate (DORA 2024) joins deployment frequency, lead time, change failure rate and time to restore. |
| D10 | Hide aggregates over very small groups? | **No.** An aggregate over one contributor is shown; revisit only if comparison charts across people or teams are added. |
| D11 | What kinds of dōjō are there? (2026-10-21, third round) | *(Revised by D20: a dōjō comes from a git integration, and an import may land as workspaces in one dōjō.)* **Two: an organization's, or a personal one.** A dōjō is the tenant — the owner. There is no hand-made dōjō: `dojo.create_dojo`, the `team` tenant origin, `team/<slug>` keys and the `team:<slug>` pack namespace are retired. |
| D12 | What is a "project"? | **A workspace** — a group of repositories. Renamed everywhere, in one slice: the database, the API, the daemon's sync, both apps and the docs (sensei and the dōjō are both pre-v1). **"Project" stops being a term.** |
| D13 | What does linking a team to a workspace do? | **Reporting only.** "Teams work on workspaces": the link says which team works on which workspace, so reports split by team. It grants nothing — access stays on repositories (D5) and viewer grants (D8). |
| D14 | Who owns an integration's credential? (2026-10-21, fourth round) | **The dōjō — one credential for the workspace.** When sensei is set up first on a developer's machine and the integration is configured there, it **hands off to the dōjō without a second configuration**; from then on the dōjō owns it and sensei reads through the dōjō. Refines D2. |
| D15 | The viewer grant model? | A team grant is **one grant per person on the team at that moment** (as built). Grants gain an **expiry** and an **export flag**. **Only an admin marks a viewer as outside the organization**, through the relationship enum (employer · client · community · personal); someone outside never exports. The **viewer shell** (`/v/{tenant}`) is **to be built**. |
| D16 | How far does D12 reach? | **Into stored values too**: the governance ladder's `project` level, pack namespaces and artifact scopes become `workspace`, converted in R3. |
| D17 | Self-hosting? | **None**, in the PRD and in code: the `self_hosted` column and the self-hosted badge go (R2). |
| D18 | Which branches does sensei read? | **Three environments — production · testing · development —** each mapped to whatever branch a repository uses (`main`/`master`, `test`/`qa`, `develop`/`dev`). Production defaults to the forge's **default branch**; an unmapped environment **falls back to production**. Release gates run in order development → testing → production (a break in testing predicts one in production); sprint targets read development. |
| D19 | Audit screens and the Diagrams list? | **Keep both** audit screens (Admin › Audit: who did what; Clients › Client audit: the confidentiality ledger). The Diagrams list is **one flat list**. |
| D20 | Where does a dōjō come from, and what does it cost? (revises D11) | **From a git integration** — a GitHub organization, an Azure DevOps organization, or another — never from nothing. When an import would make too many dōjōs, or the provider's groups fit workspaces better (Azure), the import lands **as workspaces in one dōjō**: an admin creates the dōjō by giving the org's URL, or says which imported units are dōjōs and which are workspaces. **Every dōjō gets the free tier**; billing (chosen in the org's billing page, not at creation) pays for what is beyond it — gating, integrations, consolidated metrics in the dōjō. Sensei serves the individual; shared views and gates are the dōjō's. |
| D21 | Which integration credentials go to the dōjō? (refines D14) | **A credential stays local unless its owner marks it shared.** A shared credential should be a **shared service API key** whose access covers every member of the team — a personal token carries one person's permissions and breaks when they leave. Only a shared credential hands off to the dōjō. |
| D22 | Who pulls from the tracker? | **The daemon** — each person works at their own cadence, so each daemon pulls what it needs. The dōjō may later receive **webhooks** for timely changes; until then the daemons pull. PR #274 (merged c5e515c5) is this path: the daemon's tracker tables stay. |
| D23 | How many trackers does a workspace have? | **Several, at different levels** — a tracker for an epic, separate ones for sprints, possibly one per task. A **new task's** tracker is scoped to what it changed; **incremental changes** are read against the whole system and what changed. (#274 allows one tracker per project today — to widen.) |
| D24 | What are the benefits of the paid tier? (revises the vision's "private and shared" line) | **The code is open source** — anyone can run sensei and a dōjō themselves, and it stays that way. **The hosted dōjō's paid tier adds** gating, integrations and consolidated metrics in the dōjō (D20); everything else is free. D17 still holds: we offer no self-hosting product or tier, and the hosted dōjō carries no self-hosted flag. |
| D25 | Which governance levels does a dōjō have? | **Dōjō → workspace → repository**, most specific wins. The **Team rung is retired** — teams are reporting only (D13). An individual's personal rules stay theirs, in sensei. |
| D26 | Does D10 (no small-group hiding) apply to knowledge promotion? | **No — D10 is for metric aggregates.** Knowledge promotion keeps its **k-anonymity ≥ 3** rule (T1). |
| D27 | Is Relay or push live in production? | **Not yet** — neither Relay nor push notifications has been seen on the production dōjō, and the VAPID secret is untested. Tracked as a production gap ([06-gaps](06-gaps.md#security-and-production-gaps-2026-10-21)). |
| D28 | Should relay approval gates fail closed? (#312) | **Only for the hard-block categories, only in unattended runs.** In a relay-driven autonomous run, a deploy, a push to main, a destructive action, credentials or money with no human answer is **denied**, with the reason. Interactive sessions keep fail-open — Claude Code's own prompt covers the person at the keyboard. The semantic backstop stays fail-open. |
| D29 | Are stack scopes governance levels? (D25 follow-up) | **No.** A stack is a tech stack's rules and packs (React, Rust, …), **applied at a level** — the dōjō, a workspace or a repository (user 2026-10-21). The ladder stays dōjō → workspace → repository. Client scopes are still to decide. |
| D30 | How deep does R3 go in the daemon's own database? | **Everywhere, schema too** (user 2026-10-21): `sensei.projects` and every `project_id` column are renamed in place on upgrade, rehearsed twice on a copy of a real daemon database. Another system's `project` stays (R3's rule). |
| D31 | The README frontmatter key that groups a repository into a workspace? | **Migrated in place** (user 2026-10-21): sensei reads and writes only `workspace:`, and on its next scan rewrites an existing `project:` line to `workspace:` in that README — a one-time conversion, no permanent dual-read. |
| D32 | Is Client a governance level? | **No — retired** (user 2026-10-22). A client engagement is the dōjō it lives in plus workspace classification `client`; client-only standards are rules at that dōjō or workspace. 0 rows stored. |
| D33 | Where does a converted stack pack apply? (Q-R6) | **At each repository where the stack is detected** (user 2026-10-22) — exactly today's reach; repository-rung namespaces are created as needed, and a new repository adopts by detection as today. |
| D34 | Does a dōjō keep a Clients section (Engagements, Incidents, Client audit)? | **No** (user 2026-10-22). A client is just another dōjō, and there is no client rung (D32). A dōjō's work is its Epics, Sprints and Tasks, which come from its integrations. The engagement and incident model, and the confidentiality ledger built on it, are removed. |
| D35 | How does a dōjō sign in to Azure DevOps? (R5 Q1) | **A personal access token an admin pastes** (user 2026-10-22), stored in Supabase Vault and never returned, logged or audited as a value. |
| D36 | How does an Azure DevOps organization map? (R5 Q2) | **Organization → dōjō, project → workspace** (user 2026-10-22). |
| D37 | What is R5's first slice? (R5 Q3) | **One complete source first** (user 2026-10-22, revised): R5a finishes GitHub end to end (adapter, claim by an org admin, the import/mapping screen, repositories filing by the mapping); R5b adds Azure DevOps only after that. GitLab and Bitbucket later. |
| D38 | What is the lead role for, now that Clients is gone (D34)? | **A team lead** (user 2026-10-22): a lead sees the metrics of every team member, per person, across all the workspaces the lead can access. Viewers stay aggregate-only (D6); a lead is a member, not a viewer. **The team is only the teams the lead is on** (user, 2026-10-22); a lead on no team sees only themselves. |
| D39 | How does an admin give the Integrations screen a GitHub token? (R5a) | **Re-authorize with GitHub** (user 2026-10-22): a Connect GitHub button runs GitHub OAuth again (read:org); the fresh token serves that session's reads only. It is never stored, logged or echoed. No pasted-token path. |
| D40 | D23 in practice: what is a tracker's level, and how do several show? | **A level is a separate board** (portfolio board → epics, team boards → sprints), each its own tracker tagged with its level; **a task-level link is derived** from the change (branch/commit/PR → ticket key), never configured; **several sprint trackers show side by side**, one lane per team (user 2026-10-22). Spec: docs/spec/tracker-levels.md. |
| D41 | The Teams view, per role (R8) | **An admin sees every team** with per-person rows; **a lead sees the teams they are on** with per-person rows; **contributors and maintainers see their own teams as aggregates only** (no per-person rows; a total shows only when **at least 3 people other than the reader** contributed — user 2026-10-22); **rates are weighted by volume** (each day's sessions/changes), sums stay sums; **a departed teammate drops out** of the lead's view (history stays readable to the repository's maintainers, D6) (user 2026-10-22). |
| D42 | Azure DevOps connects through Microsoft Entra OAuth (R5b; revisits D35) | A personal access token cannot list its owner's organizations, read its own scopes or report its expiry: Microsoft gates those APIs behind an Entra token. So the admin **connects with Microsoft Entra** (a delegated, authorization-code connect flow run by the dōjō server, separate from dōjō sign-in), using **one multi-tenant app registration** for sensei. The **refresh token** is kept in Supabase Vault (R5b's Vault design); access tokens are never stored. With an Entra token, the organizations list, the token's scopes (the write-scope warning) and the token's validity can be read. Organizations whose users are personal Microsoft accounts (MSA) cannot connect through Entra (Microsoft limitation), and the screen says so (user 2026-10-23). |
| D43 | Azure members match by stable id only (R5b) | A member read adds or ends only people who carry the Azure identity, matched on its stable id, never on email: an Azure principal name is not a verified email (user 2026-10-23). |
| D44 | The Azure member read runs on a schedule (R5b) | Besides import and Sync, a scheduled job reads each connected Azure organization's members and applies #309's rule (a complete read ends departed provisioned members; a partial read ends nobody) (user 2026-10-23). |
| D45 | Sign in with Microsoft (R5c, after R5b) | The login page offers **Microsoft** beside GitHub, through kavach's existing `azure` provider and Supabase Auth's Azure provider, with sign-in scopes only (`openid email profile`). A sign-in records the person's Entra object id as their Azure identity, so the member read (D43, D44) can provision and end every member of an Entra-backed Azure DevOps organization by stable id, as GitHub sign-in does for GitHub orgs. **One** multi-tenant app registration serves both sign-in and R5b's connect flow; the user creates it (user 2026-10-23). |

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

D12 renames "project" to **workspace** everywhere, and D11 retires the hand-made dōjō. Both land before
A4, so the writable workspaces are built under their real name. Sized 2026-10-21: "project" appears in
~290 crate files (~7.3k lines), ~260 app files (~2.5k), ~160 dōjō files (~1.3k), ~490 docs (~6.4k);
`sensei.projects` and ~30 related tables, views and functions; `dojo.projects` and its links. Each
slice builds, passes and ships on its own.

- **R1 · Retire the hand-made dōjō (D11).** *Done (7478fe8d).* Drop `dojo.create_dojo`, the `team` tenant origin, the
  `team:<slug>` pack namespace and the "create a dōjō" path in the dōjō UI and API; a dōjō comes only
  from an organization or a person.
- **R2 · The dōjō says workspace.** *R2a done (65eede4d): schema (renamed in place), server, `/v1` wire, the daemon's dōjō client. R2b done (f080e38a, e6c58885): page URLs, screens, nav and words; fixed the Viewers tab's R2a drift. D17's self-hosting removal done in #318 (5da293d5).* Also removes self-hosting from the dōjō (D17: `tenants.self_hosted`, the self-hosted badge). `dojo.projects` → `dojo.workspaces` with its links, viewer grants,
  team links (reporting only, D13), predicates, policies and tests; the `/v1` paths and payloads; the
  dōjō UI. The daemon's dōjō client and relay payload change in the same release (they share the wire).
- **R3 · Sensei says workspace.** *Done 2026-10-22 (merged to develop): R3a schema in place, R3b daemon + `/api/workspaces` + app + ladder rung in both DBs + D31 README key, R3c MCP tools, CLI, plugin, skills, dōjō billing. Ships in the next release.* Includes the stored values (D16): the ladder's `project` level, pack namespaces and artifact scopes, converted. `sensei.projects` and its tables, views and functions; the daemon's
  routes and pipeline; the MCP tools (`list_projects`, `use_project`, `get_project_summary`, …) and the
  plugin, skills and hooks that call them; the desktop app.
  *Sized 2026-10-21: ~13,700 occurrences — database 1.9k, daemon 7.8k, app 3.1k, MCP 0.5k, plugin 0.2k, CLI 0.2k.*
  **The rule:** rename sensei's own concept; **never another system's word.** These stay `project`:
  Claude Code's config scope (`mcp_servers.scope` `user|project`, i.e. `.mcp.json` in the repo), Jira's
  project (a board key), a forge's project (Azure DevOps), a compiler's projects (`compile_projects`:
  rust-analyzer, tsc), dbd's `project`, and English ("projection"). `extension_scope` IS sensei's word.
  Slices, each green on develop, all in one release (the app and the daemon share `/api`):
  - **R3a · database + the daemon's database layer, one change (D30).** `sensei.projects`,
    `repositories_in_projects`, `service_projects`, `extension_projects`, every `project_id` column, and the
    views, functions and indexes that name them, renamed in place by an apply/before hook (as R2a); D16's
    stored values (the ladder level, `memory_scope`, `entity_scope`, `sync_entity`, pack namespaces,
    artifact scopes) converted in the same hook; `pg_store`'s SQL and row types in the same commit, so the
    daemon stays green. Rehearse on a copy of a real daemon database, twice.
  - **R3b · daemon + app together.** `pg_store`'s names, the `/api/projects` routes → `/api/workspaces`, the
    pipeline, and the app's clients, screens and words in the same change. Also: the governance ladder's
    `project` rung (D16: `sensei.scopes.key`, `namespaces.scope_key`, the shared `scopes.jsonl` seed) converted
    in BOTH databases with the dōjō app's readers (`billing-data`, the preview rung) and the relay's
    `scope_key` in the same change, since federation needs the key on both sides; the `project_health` metric
    key and `setup.projects`; and R3a's temporary `scope_to_db` / `scope_to_api` translation removed.
    Part 1 done (d20a5b72: the daemon, `/api/workspaces`, the app). Part 2: the ladder rung, D31's README key.
    *R3a done on its branch (0ec8271c, 91894026): rehearsed twice on a copy of the daemon DB (counts held) and
    on a copy of the production dōjō (in sync, counts held).*
  - **R3c · MCP, plugin, skills, CLI.** Tool names (`list_workspaces`, `use_workspace`,
    `get_workspace_summary`, …), the plugin's hooks and skills that call them, both `SKILL.md` copies, and
    `sensei` CLI words.
- **R6 · The ladder is dōjō → workspace → repository (D25, D29, D32).** *Live in v0.18.0 (2026-10-22): the daemon DB migrated as rehearsed; production in sync. Rehearsed: rule reach identical (234 repository-rule pairs, all 87 repositories); 111 stack adoptions = 111 bindings. Note: a stack pack applied at a repository ranks at repository precedence (70), above workspace (60) — reach unchanged, precedence per D29.* Today the
  ladder still has `general · user · organization · client · technology · team · workspace · repository`.
  - **Retire `team` and `client`** (0 namespaces locally and in production): the seed, both databases (a guarded
    hook that FAILS, never deletes, if a namespace still sits on either), and every reader — the dōjō preview
    rungs (`company, client, personal, stack, workspace`), the library levels (`org · team · proj · stack`),
    `gov_scope: team` in the MCP tests.
  - **`technology` becomes packs (D29).** 19 technology namespaces on the user's machine (typescript 38 repos,
    svelte 14, java 10, python 10, …) are bound to repositories by detected stack. Each becomes a rule pack
    (its rules → `rule_pack_rules`) adopted at a level through `rule_pack_adoptions` — **adopted at each repository where
    the stack is detected (D33)** — exactly today's reach; repository-rung namespaces created as needed.
  - The library screen's levels become **dōjō · workspace · repository**; the preview shows that ladder with
    the packs applied at each rung.
  - Rehearsed on copies of both databases, counts and rule reach compared before/after.
- **R7 · A dōjō has no Clients section (D34).** *Done 2026-10-22 on branch `r7-no-clients` (not merged): the
  dōjō app, the wire (`PublishedArtifact.engagement_id`), the dōjō schema and the PRD. The release drops the
  tables through `apply/before/dojo_clients_retired.sql` (it reconciles without `--prune`) and the enums through
  `apply/after/retired_types.sql`; rehearsed twice on a copy of the production dōjō — in sync, every other count
  held. The mockups still show Clients (the user's to change). Open: `lead` now unlocks nothing a maintainer
  lacks.* Remove the Clients nav group (Engagements,
  Incidents, Client audit), their routes, screens and data code (`engagements-data`, `incidents-data`,
  `client-data`/`-map`/`-view`, engagement bindings). In the database: `dojo.engagements`, `dojo.incidents`, the
  `confidentiality_ledger` view, `is_confidentiality_action`, and `audit_events.engagement_id`, pruned through dbd
  (pre-v1: dropping data is approved). Update the PRD passages that describe engagements as a model, and the mockups.
  The daemon's own anonymization of what it contributes is separate, and stays.
- **R8 · The lead sees their team's per-person metrics (D38).** *Planned 2026-10-22, after R5a.* A lead reads
  per-person metrics (`repository_metrics` by principal, sessions, AI usage) for the members of the teams they
  lead, across every workspace the lead can access; the read policy and the lead's console view follow. Viewers
  stay aggregate-only (D6, pinned by #313's SQL test).
  *Built 2026-10-22 on branch `r8-lead-team-metrics`: `dojo.leads_team_member` + a lead clause in
  `can_read_repository_metric` (teammate's user rows, only on repositories the lead can read), the SECURITY INVOKER
  read `dojo.team_member_metrics`, `GET /v1/t/{tenant}/team-metrics` (lead floor) and the Lead → Teams view.
  Relay sessions stay owner-only (#249); the lead reads metric aggregates (sessions = `throughput`, AI usage =
  `tokens_per_day`), never sessions. Pinned by `database/tests/dojo/lead_team_metrics.sql`.
  Follow-up (user, 2026-10-22): Teams for every member — an admin sees every team with its people, a contributor
  or maintainer their teams' totals only (`dojo.team_metric_totals`, k-anonymous: at least 3 senders other than the
  reader, D26's floor counted without the caller); rates pooled
  by their denominator (Σ numerator / Σ denominator); departed teammates stay excluded. Pinned by
  `database/tests/dojo/team_metric_totals.sql`.*
  *Done 2026-10-22: merged to develop as 21a5d1a3 (D41). Walked
  through in the browser on the local dōjō for all five roles: the lead sees per-person rows for its team, m2 and c1
  see the team total only when 3 others contributed, the admin sees every team, a viewer gets 403. Ships in the next
  release. Found while walking it: #329 (the first render after a magic-link account switch uses the old session).*
- **R9 · Trackers by level (D23, D40).** *Merged to develop 2026-10-22 (branch `r9-tracker-levels`, aad7ed90): `sensei.trackers.level`
  (`epic | sprint | task`; old rows become sprint), several boards per workspace, one credential per site, per-board
  routes, `sensei tracker … --level/--board`, Integrations grouped by level, Sprints lanes side by side. An
  independent review found 6 defects (secret drain race, `--board` ambiguity across sites, lookup stopping at a
  busy board, a re-key read as gone, stale sibling errors, detail flipping between boards); each fixed red-first. Epics and
  milestones have no reader yet; that is T3's. The live check on a real Jira board (two boards, two levels) is the
  user's: the local daemon has no tracker connected.*
- **R5 · A dōjō comes from a git integration (D20).** Import from GitHub, Azure DevOps or another
  provider; when an import would make many dōjōs, or the provider's groups fit workspaces better
  (Azure), land them **as workspaces in one dōjō** — an admin creates the dōjō by giving the
  org's URL, or says which imported units are dōjōs and which are workspaces. Every dōjō starts on
  the free tier; the plan is chosen in billing. *Designed 2026-10-22: docs/spec/dojo/git-integration-import.md (D35–D37). R5a (GitHub end to end) merged to develop 2026-10-22 (8a584e51); ships in the next release. R5b (Azure) next, after R9.*
  *R5a (GitHub) built 2026-10-22 on branch `r5a-github-import` (not merged): the adapter interface, claim through
  `dojo.claim_forge_unit`, `dojo.forge_unit_mappings` + `map_forge_unit`, Admin → Integrations, and registration by
  the mapping — see the spec's "R5a as built". Production's live check waits on the merge and deploy.*
- **R5c · Sign in with Microsoft (D45).** *Queued 2026-10-23, after R5b.* Add `azure` to the dōjō's kavach providers
  and the login page (sign-in scopes only); local `supabase/config.toml` `[auth.external.azure]` and production's Azure
  provider take the app registration's client id and secret. A Microsoft sign-in records the Entra object id as an
  `azure_devops` identity, and provisioning follows the person's Azure DevOps organizations the way GitHub sign-in follows
  GitHub orgs (#309's rules). Live check: a member of a connected Entra-backed org signs in with Microsoft and lands in
  that org's dōjō; a later complete member read without them ends their membership.
- **R4 · The words.** PRD, specs, guides, `docs/llms`, both `SKILL.md` copies, agents, the website.
  Generic English ("the project's README", a Cargo project) stays.

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
