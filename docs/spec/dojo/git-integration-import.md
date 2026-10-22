# A dōjō comes from a git integration — design (#308, R5, D20)

**Status:** design for review (2026-10-22). No code until the open questions below are answered.

## The decision this serves

D20: a dōjō comes from a **git integration** — a GitHub organization, an Azure DevOps organization, or
another provider — never from nothing. A client is just another dōjō (D34). Every dōjō starts on the
free tier; the plan is chosen in billing (D24).

## What exists today

- **GitHub only, on sign-in.** `provisioning.ts` `ensureProvisioned` creates the person's personal dōjō
  and one `organization/<login>` dōjō per GitHub org the person belongs to (`forge-github.ts`,
  `fetchGithubOrgs`, paged and completeness-tracked since #309). It never removes; a complete org read
  ends a provisioned membership the forge no longer lists (#309).
- **`dojo.tenant_connections`** links a dōjō to a forge org: `provider` (`github | gitlab | bitbucket |
  azure_devops`), `external_id` (the forge's stable id, NULL until proven), `external_slug`,
  `verified_at`. The enum already names four forges; only GitHub is wired.
- **`repo-mapping.ts`** maps a repository key (`host/org/repo`, normalised by the daemon) to a forge org,
  including Azure's `v3` SSH form. An unrecognised host stays **unmapped**, never personal.
- An org dōjō is **unclaimed** until someone proves control (no `claimed_at` yet).

## The model

1. **A unit** is what a provider groups repositories by: a GitHub org; an Azure DevOps organization and,
   inside it, its projects; a GitLab group and subgroups.
2. **An import** reads the units a person can see through their connection and proposes a mapping.
   Each unit becomes either **a dōjō** or **a workspace inside a dōjō**:
   - GitHub org → dōjō (as today).
   - Azure DevOps organization → dōjō; each of its projects → a workspace in it (Q2).
   - A person with many small units may fold them into one dōjō as workspaces.
3. **Proof of control.** An org stays forge-proven (decision 2026-10-12): the person creating or claiming
   the dōjō must be an admin of the unit at the provider, checked with their own token at import time
   and recorded in `tenant_connections.verified_at`. A unit that cannot be proven can still be imported
   as a workspace of a dōjō the person already administers, never as a new dōjō.
4. **Repositories follow the mapping.** `repo-mapping.ts` resolves a repository to its unit; the unit's
   mapping says which dōjō and workspace it files into. Unmapped stays unmapped.
5. **Membership** follows the provider's member list (as #309 does for GitHub): provisioned members end
   when a complete read no longer lists them; invited members are never touched.

## Provider adapters

One adapter per provider, behind one interface: `listUnits(token) → {units, complete}`,
`proveAdmin(token, unit) → bool`, `listMembers(token, unit) → {members, complete}`. "Complete" is
mandatory: a partial read never ends a membership (#309's rule).
- **GitHub:** exists; refactor `forge-github.ts` behind the interface.
- **Azure DevOps:** organizations via the accounts API, projects per organization, admin via the
  organization's Project Collection Administrators group. Auth is Q1.
- **GitLab, Bitbucket:** later (Q3).

## Screens

- **Admin → Integrations → Connect an organization:** pick a provider, sign in or paste a token (Q1),
  see the units with a proposed mapping (dōjō / workspace of …), adjust, import. Units the person
  cannot prove are marked and can only become workspaces of a dōjō they already administer.
- **The personal dōjō** offers the same import for units the person owns.

## Open questions (the user's)

- **Q1 · Azure DevOps sign-in:** an Entra ID OAuth app (proper consent, refreshable, works for every org
  the person can see — *recommended*) or a personal access token pasted by an admin (simplest, no app
  registration, but long-lived secrets and per-org scope)?
- **Q2 · Azure default mapping:** organization → dōjō and project → workspace (*recommended*), or each
  project → its own dōjō?
- **Q3 · First slice:** GitHub refactor + Azure DevOps only (*recommended*), with GitLab and Bitbucket
  later — or all four at once?

## Done gate

- An Azure DevOps organization admin imports their organization: one dōjō, one workspace per project,
  repositories filed by `repo-mapping`; a non-admin cannot create that dōjō.
- A complete member read ends a departed provisioned member; a partial read ends nobody.
- A GitHub org still provisions on sign-in exactly as today.
