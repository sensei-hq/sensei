---
title: dōjō access matrix — who sees and changes which data
description: Role × data area, each cell own / repository / viewer / tenant / none for read and write. The policies (views + RLS) derive from this; it is not reverse-engineered from today's 167 service-role calls.
date: 2026-10-12
status: approved (#236 step 1)
---

# dōjō access matrix

**Read with** [`docs/architecture/entity-access-model.md`](../../architecture/entity-access-model.md),
which fixes the AXIS — the person (via memberships) for personal work, the tenant
for governance and org reads. This document assigns the CELLS under that axis and
defers to it where they meet.

## Why this exists

Every dōjō data call runs as `service_role`, which bypasses RLS. Authorization
is re-decided in code beside each of 167 calls. That was a recorded beta
shortcut (`docs/history/plan/decisions.md`, "Relay RLS": RLS deferred "when multi-user
tenants need row isolation"). That trigger is met.

**Decision B (2026-10-08, #236):**
- The dōjō reads user-visible data **as the signed-in user**, through **views
  with RLS**, never raw tables.
- Individuals see their own data.
- Leads and management additionally see cross-user data for their workspaces.

This document is the single statement of who may do what. Policies, views and
the route moves are derived from it, and a cell here that disagrees with a policy
is a bug in the policy.

## Who

| Principal | Defined by | Notes |
|---|---|---|
| **anyone signed in** | a valid session JWT → `dojo.current_principal_id()` | Not a member of anything. The matrix gives this nothing tenant-scoped. |
| **member** | an ACTIVE membership in the tenant (`memberships.disabled_at is null`) | The floor for every tenant-scoped cell. A missing or disabled membership is **no access**, never "level 0". |
| **contributor · maintainer · lead · admin** | `memberships.role` (`dojo.member_role`) | The TENANT role. `admin` is "management": policy, identity, billing, provisioning. A tenant-role `lead` sits between maintainer and admin and is never derived from the forge. Since D38 (#307 R8) it is the **team lead**: it reads the per-person metrics of the people it shares a team with, on the repositories it can read (Metrics, below). |
| **team** (#307, D7, D13) | `dojo.team_members`; `dojo.team_projects` links a team to the workspaces it works on | **Grants no repository access.** It decides one thing: whose per-person metrics a tenant `lead` reads (D38) — never where. A grouping for "by team" reports, and a way to grant workspace viewer to everyone on the team at once (one grant per person at that moment, D15). The link to a workspace is reporting only (D13). A workspace's relationship to access is its repositories (`dojo.repositories_in_projects`). |
| **workspace viewer** (#307, D8) | a viewer grant on one workspace | **Read-only managerial view** of the workspace — metrics, dashboards, reports, aggregated or split by repository; never per-person. Admin-granted, only to someone already in the dōjō — a member or a dōjō viewer. Grants nothing else. |
| **dōjō viewer** (#307, D8 revised) | an accepted invite with `access = viewer` (`dojo.dojo_viewers`) | **Nothing by itself** — not a membership; reads no repository. It makes the person eligible for workspace viewer grants in that dōjō. Removing them also removes their workspace views there (unless they are also a member). The dōjō's admins list them (with the email they were invited at) and read their identities. |
| **repository role** (#307, D5) | the person's role on one repository — mirrored from the forge, admin-overridable | **Where access lives.** A repository's shared rows for anyone with a role on it; per-person rows for the person and maintainers and above on it (D6). |
| **daemon** | the signed-in persona's session, presented by the daemon for the memberships that persona joined (step 4) | Same principal, same matrix. There is no device credential: any other bearer is a 401. |

**The tenant roles are ordered** `contributor < maintainer < lead < admin` —
`dojo.member_role`'s declared order and the entity model's. The dōjō's code
ladder is pinned to that order by `dojo-access-ladder.spec.ts`.

**Superseded 2026-10-21 (#307 D5–D8).** Access moves from team membership to **repository roles**; teams become groupings with no access meaning, and a workspace's own permission is the read-only viewer grant. The predicates are rewired (#307 A2b): `can_read_repository` (any role on the repository), `can_read_repository_metric` (another person's rows: maintainer and above on the repository), `peer_run_status` (a role on one of the workspace's repositories), `can_read_project` (also its viewers, and its dōjō's admins — so a workspace is visible to them before it holds a repository); `on_team_of` / `leads_team_of` are retired. The paragraph below records the earlier decision and what replaced it.

**Earlier (2026-10-12): two leads.** The tenant role `lead` (client work) and a
team role `lead` (cross-user workspace data) answered different questions, and
the team role was chosen for cross-user data. **Replaced by D5–D7 (2026-10-21):**
cross-user data follows the repository role — maintainers and above on a
repository see its per-person rows (D6) — and teams grant nothing. There is one
`lead`: the tenant's confidentiality role.

## Scopes

| Scope | Rows |
|---|---|
| **own** | keyed to the caller's principal, or to ANY membership the caller owns (`dojo.owns_membership`). On a tenant-addressed route, a membership of that tenant; on `/you`, every dōjō the person belongs to — personal work is not filtered by one tenant ([entity & access model §3](../../architecture/entity-access-model.md)) |
| **repository** | rows of repositories the caller holds a role on (`repo_role_at_least(repository, 'contributor')`), and the workspaces that hold one of them |
| **repository-maintainer** | the same repositories, restricted to those where the caller's repository role is `maintainer` or above |
| **viewer** | one workspace the caller was granted as viewer: aggregates and per-repository splits, never per-person (D8) |
| **tenant** | every row of the tenant |
| **none** | no access |

## The matrix

`R` = read, `W` = insert/update/delete. A cell names the WIDEST scope the role
gets. Roles inherit the cells of the roles below them unless a cell says otherwise.

### Relay — what a person's agents are doing

| Data | member | repository role | admin |
|---|---|---|---|
| sessions, segments, inbox (gates, replies, content) | **R/W own** | — | count only (health) |
| run status (a task is running on workspace P) | own | **R repository** — runs on a workspace holding a repository you have a role on (status, never content; `peer_run_status`) | R tenant (status) |
| push subscriptions, notification prefs | R/W own (requires membership) | — | — |

Relay content is the person's machine. **No role reads another person's
responses, gates or replies**, admin included (#249). Peer status is a separate,
content-free view (#252).

### Workspaces and repositories

| Data | member | repository role | workspace viewer | admin |
|---|---|---|---|---|
| workspaces (rows, constitution) | — | **R repository** (a workspace holding one of your repositories; `can_read_project`) | R the granted workspace | R tenant |
| repositories (catalogue) | R own (`my_repositories`); W register (daemon) | R repository | — (per-repository splits only, in aggregates) | R/W tenant |
| repository elections (share decisions) | R/W own (user authority) | — | — | R/W tenant (org authority) |
| repository visibility | written by the system from the forge, never by a member's request | | | |

A workspace has no owner (#299). Workspaces are written by the sync today;
creating and editing them in the dōjō is #307 A4.

### Metrics

| Data | member | repository role | repository maintainer+ | workspace viewer | admin |
|---|---|---|---|---|---|
| repository metrics, `scope = repo` | — | R repository | R repository | aggregates and per-repository splits of the granted workspace | R tenant |
| repository metrics, `scope = user` | R own | R own | **R repository-maintainer (cross-user, D6)** | never per-person | R tenant |
| repository metrics, `scope = user`, as a tenant **lead** (D38) | R own | R own | **R teammates' rows on repositories the lead can read** | never per-person | R tenant |
| metric activations (tenant-wide on/off) | — | — | — | — | **W** (org policy) |
| metric activations (own) | W own, user-authority repositories | | | | |

`dojo.can_read_repository_metric` grants a person's own user-scoped rows,
repo-scoped rows to anyone with a role on the repository, another person's rows
to a maintainer or above on it (D6), and the whole tenant to its admins (#307
A2b). The viewer's aggregate read is decided (D8) and not yet in the predicate.

**A tenant lead reads their team (#307 R8, D38).** `dojo.leads_team_member(person,
tenant)` is true when the caller's active membership in the row's dōjō is `lead` or
above and the caller shares a team **of that dōjō** with the person, who is an
active member there. `can_read_repository_metric` grants a `scope = user` row when
that holds **and** the caller can read the row's repository (`can_read_repository`
— a role on it, D5). So: a lead on no team reads only their own rows; a teammate's
row on a repository the lead holds no role on stays hidden; a viewer is never a
member and reads none (D6, `viewer_audit.sql`). Relay sessions stay owner-only
(#249): the lead reads a person's metric aggregates, never their sessions. The
console reads through `dojo.team_member_metrics` (SECURITY INVOKER — the row
policy decides), pinned by `database/tests/dojo/lead_team_metrics.sql`.

**A person reads their own series (developer console, 2026-10-26).**
`dojo.my_metric_series(since)` returns the caller's own daily `scope = user` rows,
one per metric and day, pooled across their repositories like the lead's view
(counts summed, rates by their denominators). It is SECURITY INVOKER **and** bound
to `current_principal_id()`, so an admin or lead who may read others' rows still
gets only their own series. Pinned by `database/tests/dojo/my_metric_series.sql`.

**Every member reads their teams' totals (#307 R8, 2026-10-22).** The Teams view
serves every member: per-person rows for a lead (their teams) and an admin (every
team); for a contributor or maintainer, team totals only. `dojo.team_metric_totals`
(SECURITY DEFINER, caller from the session) sums counts and pools rates by their
own denominator (Σ value × denominator / Σ denominator) over the team's active
members' rows **on repositories the caller can read**, for the teams the caller is
on (every team for an admin). Below lead it answers a (team, metric) only when at
least `dojo.k_anonymity_min()` = **3 people other than the reader** sent it (user,
2026-10-22) — the dōjō's k-anonymity floor (D26, the knowledge gate's
`K_ANONYMITY_MIN_CONTRIBUTORS`), counted without the caller, so a member who
subtracts their own number is still left with at least 3 colleagues. The constant
stays 3; "others" is the function not counting the caller. A lead or admin reads
per person anyway, so for them the floor counts everyone. This is a deliberate
exception to D10 (no small-group hiding) for team totals shown to members, at the
user's direction. Pinned by `database/tests/dojo/team_metric_totals.sql`.

A `scope = user` row arrives only as its own person's: `dojo.file_metric_rows`
takes `principal_id` from the session, never the input, and the daemon sends only rows
provably the signed-in person's (#240; `docs/spec/dojo/daemon-sync.md` §8, item 4).

### Knowledge — artifacts, triage, rules, policies

| Data | contributor | maintainer | admin |
|---|---|---|---|
| artifacts, own submissions | R/W own | | |
| artifacts, others' submissions (pre-triage) | — | R tenant | R tenant |
| artifacts, published | R tenant | R tenant | R tenant |
| triage queue, decisions | — | **R/W tenant** | R/W tenant |
| shared rules | R tenant | **W tenant** (publish, retract, overwrite) | W tenant |
| policies | R tenant | R tenant | **W tenant** |

**A shared rule is owned by one tenant** (decision 2026-10-12): `shared_rules`
gains `tenant_id`. Sharing across tenants becomes an explicit publish to a
shared namespace later, never an accident of an unscoped table.

A contributor's publish may trigger clustering (`promoteCluster`) that changes
OTHER members' submissions. That is a maintainer action and moves behind the
maintainer cell, as a system step with an audit row.

### Audit

| Data | lead (tenant role) | admin |
|---|---|---|
| audit events | — | R tenant |

A dōjō has no Clients section (#307 R7, D34): a client is just another dōjō, so
there are no engagements, incidents or confidentiality ledger to read.

### Tenancy, identity, billing

| Data | member | admin |
|---|---|---|
| own membership, own principal | R own | |
| memberships of the tenant | — | R/W tenant (adding a person requires their acceptance — invites) |
| identities | R own | **R tenant members' identities; never W.** An identity is the PERSON's and spans tenants. Binding a forge identity requires that forge's proof (provisioning). |
| seats, billing account, invites, health | — | R/W tenant |
| forge unit mappings (where a GitHub org's repositories file, #308 R5a) | — | R tenant (`forge_unit_mappings`); W through Integrations only (`map_forge_unit`, after the forge proved what the admin may do) |

## What stays `service_role`

The explicit exceptions. Each is system work with no user in the loop; each
writes an audit row where it changes tenant state.

| Work | Why no user session |
|---|---|
| provisioning on sign-in / CLI token exchange (principals, forge-proven tenants, memberships, identities) | establishes the principal the session would need |
| forge sync (repository visibility, org membership) | the forge is the authority; no member decides it |
| Integrations: claim an org's dōjō, map an org (`claim_forge_unit`, `map_forge_unit`, #308 R5a) | the forge is the authority on who administers the org — the server reads it with the person's own token, then the function records what that proved; both audited |
| billing webhooks, seat opening from relay activity | external caller / system bookkeeping |
| invite acceptance | the joining principal has no membership yet; the invite token is the authority |
| clustering after a publish (`promoteCluster`) | derived maintenance over many members' rows |
| the audit row for a person's action (`recordAudit`) | the system's record of what the person did, written from the resolved caller; a member never authors ledger rows |
| a display name for an id on a row the caller already read (`resolveDisplayNames`, e.g. a member's or a team member's name; a seat's workspace name) | names live on memberships/identities/`sensei.namespaces`, which a session does not read; the lookup resolves only the ids the caller's own read returned |
| the `seats_used` refresh (`POST …/billing`) | a money-facing cache derived from the seats; the system recomputes it after the admin gate, and no session writes the number |

Everything not in this table moves to the user session.

## When a session needs facts it may not read

Some answers a person is entitled to are computed from rows they may not read —
their repository's sync verdict needs the tenant's billing account; registering a
repository needs every tenant's forge connections to say "not yours" rather than
"unknown". Granting the session those tables would widen the matrix. Instead, a
`security definer` function reads them and returns only the caller's answer:

- its principal comes from the session (`dojo.current_principal_id()`), **never
  from an argument** — a definer function that takes a user id serves anyone who
  can call it;
- it returns the caller's rows or a reason code, never another tenant's ids;
- it refuses with a SQLSTATE naming why (`no_data_found` 404,
  `object_not_in_prerequisite_state` 409, `insufficient_privilege` 403), which the
  route keeps as that status (`db-refusal.ts`).

Today: `my_repositories`, `register_repository`, `set_repository_election`,
`file_metric_rows`, `set_metric_activation`, `set_pack_adoption`,
`my_adopted_pack_slugs`, `tenant_live_session_count`, `principal_is_member`,
`leads_team_member`.
Each has a test under `database/tests/dojo/`. (`create_dojo` was dropped in R1,
7478fe8d — D11/D20.)

## Sequence

1. **This matrix.**
2. **Helpers + views + RLS, with policy tests.** One `security definer`
   predicate per relationship: `is_member(tenant)`, `tenant_role_at_least(tenant,
   role)`, `repo_role_at_least(repository, role)`, `can_read_repository`,
   `can_read_project` (`on_team_of` / `leads_team_of` were retired by #307 A2b;
   `project_of(repository)` was never built).
   Views are `security_invoker`, so the base tables' policies decide. Every cell
   gets a test pair: the cell's role CAN, the role below it CANNOT, and a member
   of another tenant cannot.
3. **Move the routes**, one area at a time, from `dojoDb()` to the user-session
   client (`callerDb`, `asOwner`, `sessionDb`). **Done for every JWT-plane route
   (2026-10-12)**; what still runs as the service role is the table above, and the
   device plane below.
4. **The daemon as the person** — **done (2026-10-13)**. The daemon's tenant
   calls present the signed-in persona's session, so the same policies apply to
   it; the device token, `resolveApiKeyAccess` and
   `dojo.memberships.device_token_hash` are gone (step 4e). Signing in is joining:
   the daemon creates its membership from `GET /v1/you/memberships`.

## Related

- #236 (this work), #249 (relay owner-only), #252 (peer status), jerrythomas/kavach#51
- `database/policies/dojo/*.sql`, `database/ddl/function/dojo/*.ddl` — today's policies and helpers
- Findings from the 2026-10-12 inventory are tracked as private security advisories on this repository, not here.
