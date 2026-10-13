# Dōjō Consoles · T2

**Plane:** team · **Status:** [roadmap/04-team.md](../../../../roadmap/04-team.md) ·
**Serves:** team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
org admin ([persona 4](../../../03-personas.md#4--org-admin--the-plane-owner)),
engagement / client lead ([persona 5](../../../03-personas.md#5--engagement--client-lead--the-confidentiality-guard))

## Purpose

The Dōjō Consoles are the **four role-scoped surfaces** (developer · maintainer ·
admin · lead) that make the Dōjō usable for each person's job. They are **additive
grants**, not a single admin/not switch — a maintainer (governance) needn't be an
admin (roles/policies), and vice-versa. The personal landing ("your work") is
always primary; the consoles are what you open *inside* an org context. The job
is **each role sees exactly what it can act on** — no more, no less. The user
feels this as *I open my org and the surfaces match my role — I don't trip over
things I can't touch.*

> **Status honesty.** The data layer exists (memberships, roles, `dojo.member_role`
> enum, role ladder in `dojo-auth.ts`), the personal screens (`/you/*`, Relay,
> Library, Preview) are built and tested. The **org context `/org/[slug]` with its
> own nav and the four role-scoped surfaces are unbuilt** (gap T-2). The current
> `/console/*` screens are a flat shell — they don't role-gate. The org-switcher
> popover is built (top bar: "Relay · you — all Dōjōs" + one row per membership +
> "Your Dōjōs" + "＋ Create or join").

## In scope / out of scope

- In: the four role definitions and their additive grants:
  - **Developer** (default) — read-mostly: see projects in jurisdiction, preview
    a project's effective constitution, view members.
  - **Maintainer** — **manage governance at org level**: author rules/stance on
    the ladder, triage/approve findings, manage knowledge.
  - **Lead** — client engagements, confidentiality, audit trail.
  - **Admin** — **change member roles + access policies**: members, identities/SSO,
    scopes & precedence, billing.
  - The org context `/org/[slug]` with its own nav pane (projects, governance,
    role surfaces).
  - The personal screens that are already built: `/you/projects`, `/you/inbox`,
    `/you/relay`, `/library`, `/preview`, `/me`.
- Out: the federation backbone (T1), project planning against trackers (T3),
  billing/metering (T4).

## Surfaces

- **App (target routes)** — `/org/[slug]` (org home: projects in jurisdiction);
  `/org/[slug]/governance` (ladder config — maintainer);
  `/org/[slug]/projects/[id]` (project preview: which ladder levels built the
  constitution);
  `/org/[slug]/members`, `/identities`, `/policies`, `/billing` (admin);
  `/org/[slug]/triage`, `/approvals`, `/knowledge` (maintainer);
  `/org/[slug]/engagements`, `/incidents`, `/audit` (lead).
- **App (built personal)** — `/` (personal landing: your work + My Dōjōs);
  `/you/projects`, `/you/inbox`, `/you/relay`, `/library`, `/preview`, `/me`.
- **Daemon** — `dojo-auth.ts` (`resolveTenantAccess`, role/access-floor logic);
  `governance.rs` (resolution, cached 5 min); `get_rules` / `get_layered_context`
  MCP tools.
- **Data** — `dojo.memberships` (user_id, org_id, role, kind), `dojo.member_role`
  enum (`member < contributor < lead < maintainer < admin`), `dojo.shared_rules`,
  `dojo.tenants`, `dojo.audit_ledger`.

## Data invariants

- **Lead.** Grants are **additive** — the union of a user's roles is what they
  see. A maintainer + admin sees both governance and roles/policies.
- **Lead.** The personal landing is **always reachable** — the org-switcher
  popover has "Relay · you — all Dōjōs" pinned at top.
- **Lead.** Role floor: `member < contributor < lead < maintainer < admin`.
  A role reveals its surfaces; the union is what you see.
- **Lead.** Client binding **excludes employer** — the owning org wins ordinary
  conflicts; the employer's mandatory rules remain a floor.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Org context nav | projects / governance / members / triage / engagements | maintainer sees governance + triage |
| Role badge | the user's role in this org | `maintainer` |
| Governance authoring | stance + rules/skills/agents/commands + project memory | maintainer writes `enforcement: mandatory` |
| Triage queue | findings awaiting review | 3 pending (2 auto-approved, 1 human) |
| Audit trail | immutable log | `promote rule#r-12 by u-3 at 2026-10-13T14:22:00Z` |
| My Dōjōs section | orgs + role on personal landing | `Sensei-HQ · maintainer` |

## Done gate

- A maintainer opens `/org/[slug]/governance` and can author a stance (autonomy,
  sharing, review, anonymization) + rules/skills/agents/commands for that org's
  ladder rung; it cascades down and appears in `get_rules` after cache expiry.
- An admin opens `/org/[slug]/members` and changes a user's role; the access
  floor updates immediately in `resolveTenantAccess`.
- A lead opens `/org/[slug]/engagements` and sees client engagements with
  confidentiality pinned; the audit trail is immutable.
- A developer opens `/org/[slug]` and sees projects in jurisdiction; clicking
  one drills to `/org/[slug]/projects/[id]` previewing which ladder levels built
  the constitution.
- The personal landing (`/`) shows "your work" (projects, needs-you, relay) as
  primary and "My Dōjōs" with role badges as secondary.

```bash
# Worker API (requires auth)
curl -s -H "Authorization: Bearer $TOKEN" \
  "https://dojo.sensei-hq.com/v1/t/org/sensei-hq/members" | jq '.[] | {user_id, role}'
```

## Wrong gate

- **A maintainer sees the admin surfaces (members/roles/policies)** — role-gating
  regressed.
- **An admin cannot change a member's role** — `resolveTenantAccess` regressed.
- **A developer can author governance** — role-gating regressed.
- **The personal landing shows the management overview instead of "your work"**
  — work-first regressed.
- **Clicking an org in My Dōjōs goes to `/console/teams` instead of
  `/org/[slug]`** — org context regressed.
- **A lead's audit trail is mutable or missing events** — audit regressed.

## Scenarios

- [`scenarios/dojo-consoles.feature`](scenarios/dojo-consoles.feature) — the
  four role consoles and their surfaces (pinned for built personal screens,
  `NO TEST` / `# gap: T-2` for org context).
- [`scenarios/dojo-consoles-gaps.feature`](scenarios/dojo-consoles-gaps.feature) —
  the unbuilt org context role surfaces (`NO TEST` / `# gap: T-2`).

## Open gaps

- [T-2](../../../../roadmap/06-gaps.md) — join/membership flow: magic-link
  creates `auth.users` but no membership; "Join" dead → every console surface
  403s (P1). Blocks T2 consoles.
- [T-1](../../../../roadmap/06-gaps.md) — T3's tracker adapters (Jira / ADO)
  exist only in mockup (P1).

## Related

- [design/dojo-web.md](../../../../design/dojo-web.md) — sitemap, routes & IA
  (roles table, org context nav, target routes).
- [design/governance.md](../../../../design/governance.md) — identity,
  membership & routing; role ladder.
- [journeys/dojo.md](../../../../journeys/dojo.md) — journey map for each role.
- [mockups/Sensei/lib/dojo/](../../../../mockups/Sensei/lib/dojo/) — visual
  source (never edited): `dojo-saas.jsx`, `dojo-governance.jsx`,
  `dojo-preview.jsx`, `dojo-billing.jsx`.
- [prd/02-objectives.md §Dōjō](../../../02-objectives.md) · [prd/03-personas.md §2,3,4,5](../../../03-personas.md).
- Crates: `crates/senseid/src/dojo/`; Worker: `dojo/src/routes/(app)/org/*`,
  `dojo/src/routes/(app)/you/*`, `dojo/src/lib/server/dojo-auth.ts`.