---
title: dōjō access matrix — who sees and changes which data
description: Role × data area, each cell own / team / tenant / none for read and write. The policies (views + RLS) derive from this; it is not reverse-engineered from today's 167 service-role calls.
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
shortcut (`docs/plan/decisions.md`, "Relay RLS": RLS deferred "when multi-user
tenants need row isolation"). That trigger is met.

**Decision B (2026-10-08, #236):**
- The dōjō reads user-visible data **as the signed-in user**, through **views
  with RLS**, never raw tables.
- Individuals see their own data.
- Leads and management additionally see cross-user data for their projects.

This document is the single statement of who may do what. Policies, views and
the route moves are derived from it, and a cell here that disagrees with a policy
is a bug in the policy.

## Who

| Principal | Defined by | Notes |
|---|---|---|
| **anyone signed in** | a valid session JWT → `dojo.current_principal_id()` | Not a member of anything. The matrix gives this nothing tenant-scoped. |
| **member** | an ACTIVE membership in the tenant (`memberships.disabled_at is null`) | The floor for every tenant-scoped cell. A missing or disabled membership is **no access**, never "level 0". |
| **contributor · maintainer · lead · admin** | `memberships.role` (`dojo.member_role`) | The TENANT role. `admin` is "management": policy, identity, billing, provisioning. A tenant-role `lead` guards client confidentiality (engagements, incidents, audit ledger). |
| **team member / team lead** | `dojo.team_members` (+ `.role = 'lead'`) on a team that owns the project (`dojo.team_projects`) | The PROJECT relationship. "A lead's projects" means projects owned by a team on which the principal's team role is `lead`. |
| **device** | a daemon's device token, exchanged for a short-lived JWT scoped to its principal (step 4) | Same principal, same matrix. Until step 4 the device plane stays in code. |

**The tenant roles are ordered** `contributor < maintainer < lead < admin` —
`dojo.member_role`'s declared order and the entity model's. The dōjō's code
ladder is pinned to that order by `dojo-access-ladder.spec.ts`.

**Two leads, on purpose.** The tenant role `lead` (client work) and the team
role `lead` (cross-user project data) answer different questions. The second
was chosen over the tenant role on 2026-10-12. **Consequence:** nothing in the
dōjō creates teams today, so until team management exists **no one sees
cross-user project data except `admin`**. That is the correct reading of the
decision, and the gap is a missing feature, not a policy to loosen.

## Scopes

| Scope | Rows |
|---|---|
| **own** | keyed to the caller's principal, or to ANY membership the caller owns (`dojo.owns_membership`). On a tenant-addressed route, a membership of that tenant; on `/you`, every dōjō the person belongs to — personal work is not filtered by one tenant ([entity & access model §3](../../architecture/entity-access-model.md)) |
| **team** | belonging to a project owned by a team the caller is on |
| **team-lead** | the same projects, restricted to teams where the caller's team role is `lead` |
| **tenant** | every row of the tenant |
| **none** | no access |

## The matrix

`R` = read, `W` = insert/update/delete. A cell names the WIDEST scope the role
gets. Roles inherit the cells of the roles below them unless a cell says otherwise.

### Relay — what a person's agents are doing

| Data | member | team member | team lead | maintainer | admin |
|---|---|---|---|---|---|
| sessions, segments, inbox (gates, replies, content) | **R/W own** | — | — | — | count only (health) |
| run status (a task is running on project P) | own | **R team** (status, never content) | R team | — | R tenant (status) |
| push subscriptions, notification prefs | R/W own (requires membership) | — | — | — | — |

Relay content is the person's machine. **No role reads another person's
responses, gates or replies**, admin included (#249). Peer status is a separate,
content-free view (#252).

### Projects and repositories

| Data | member | team member | team lead | admin |
|---|---|---|---|---|
| projects (rows, constitution) | R/W own | R team | R team-lead (cross-user detail) | R tenant |
| repositories (catalogue) | R repositories of own projects; W register (daemon) | R team | R team | R/W tenant |
| repository elections (share decisions) | R/W own (user authority) | — | — | R/W tenant (org authority) |
| repository visibility | written by the system from the forge, never by a member's request | | | |

### Metrics

| Data | member | team member | team lead | admin |
|---|---|---|---|---|
| repository metrics, `scope = repo` | R repositories of own projects | R team | R team | R tenant |
| repository metrics, `scope = user` | R own | — | **R team-lead (cross-user)** | R tenant |
| metric activations (tenant-wide on/off) | — | — | — | **W** (org policy) |
| metric activations (own) | W own, user-authority repositories | | | |

`dojo.can_read_repository_metric` today grants own, repo-scope and tenant admin.
The team-lead clause is new.

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

### Client work and compliance

| Data | lead (tenant role) | admin |
|---|---|---|
| engagements, incidents | **R/W tenant** | R/W tenant |
| audit ledger (confidentiality actions) | R tenant | R tenant |
| audit events (full) | — | R tenant |

References written into these rows (`client_tenant_id`, `owner_id`,
`engagement_id`, `artifact_id`) must name objects of the same tenant (or, for
`client_tenant_id`, a tenant with a recorded connection). An unvalidated
reference is a read through a side door.

### Tenancy, identity, billing

| Data | member | admin |
|---|---|---|
| own membership, own principal | R own | |
| memberships of the tenant | — | R/W tenant (adding a person requires their acceptance — invites) |
| identities | R own | **R tenant members' identities; never W.** An identity is the PERSON's and spans tenants. Binding a forge identity requires that forge's proof (provisioning). |
| seats, billing account, invites, health | — | R/W tenant |

## What stays `service_role`

The explicit exceptions. Each is system work with no user in the loop; each
writes an audit row where it changes tenant state.

| Work | Why no user session |
|---|---|
| provisioning on sign-in / CLI token exchange (principals, forge-proven tenants, memberships, identities) | establishes the principal the session would need |
| forge sync (repository visibility, org membership) | the forge is the authority; no member decides it |
| billing webhooks, seat opening from relay activity | external caller / system bookkeeping |
| invite acceptance | the joining principal has no membership yet; the invite token is the authority |
| clustering after a publish (`promoteCluster`) | derived maintenance over many members' rows |

Everything not in this table moves to the user session.

## Sequence

1. **This matrix.**
2. **Helpers + views + RLS, with policy tests.** One `security definer`
   predicate per relationship: `is_member(tenant)`, `tenant_role_at_least(tenant,
   role)`, `on_team_of(project)`, `leads_team_of(project)`, `project_of(repository)`.
   Views are `security_invoker`, so the base tables' policies decide. Every cell
   gets a test pair: the cell's role CAN, the role below it CANNOT, and a member
   of another tenant cannot.
3. **Move the routes**, one area at a time, from `dojoDb()` to the user-session
   client. Data routes where they fit; rpc routes once jerrythomas/kavach#51 is
   fixed.
4. **The device plane**: device token → short-lived JWT for its principal, so the
   same policies apply to the daemon.

## Related

- #236 (this work), #249 (relay owner-only), #252 (peer status), jerrythomas/kavach#51
- `database/policies/dojo/*.sql`, `database/ddl/function/dojo/*.ddl` — today's policies and helpers
- Findings from the 2026-10-12 inventory are tracked as private security advisories on this repository, not here.
