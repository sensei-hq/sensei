# A dōjō comes from a git integration — design (#308, R5, D20)

**Status:** decided 2026-10-22 (Q1–Q3 answered by the user: D35–D37). Ready to build.

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

## Decisions (user, 2026-10-22)

- **D35 · Azure DevOps sign-in is a personal access token** an admin pastes (no app registration).
- **D36 · Azure maps organization → dōjō, project → workspace** by default.
- **D37 (revised, user 2026-10-22) · One complete source first.** **R5a** finishes the flow for GitHub end to
  end; **R5b** adds Azure DevOps on the same adapter and screen only after R5a is done and verified. GitLab
  and Bitbucket later.

## R5a · GitHub, complete

1. The adapter interface (`listUnits`, `proveAdmin`, `listMembers`, each with `complete`); `forge-github.ts`
   moves behind it unchanged in behaviour.
2. Proof of control: an org admin (GitHub org role `admin`) claims the org's dōjō — `claimed_at`,
   `claimed_by`, `tenant_connections.verified_at`; an unclaimed org dōjō shows who can claim it.
3. The import screen (Admin → Integrations): the GitHub orgs the person can see, each mapped to **its own
   dōjō** (today's default) or **a workspace in a dōjō they administer**; units they cannot prove can only
   become workspaces.
4. Repositories file by the mapping (`repo-mapping.ts` → unit → dōjō + workspace); unmapped stays unmapped.
5. Unchanged: provisioning on sign-in, and #309's membership lifecycle.
Done when an org admin, on the local and then the production dōjō, claims an org, maps a second org as a
workspace of it, and its repositories file there.

### R5a as built (2026-10-22, branch `r5a-github-import`)

- **Adapter** — `dojo/src/lib/server/forge-adapter.ts`: `ForgeAdapter { provider, listUnits, proveAdmin,
  listMembers }`, `forgeAdapter(provider)` (501 for a forge with no adapter). GitHub's sits in `forge-github.ts`:
  `listUnits` is the org-membership read provisioning already used (unchanged); `proveAdmin` reads
  `/user/memberships/orgs/{org}` — true only for an active `admin` of the org with the known stable id, `false` on
  404, throws on any other failure; `listMembers` pages `/orgs/{org}/members`, incomplete on an unnamed member,
  throws on a failed page or more than 5,000. A unit is an org (`ForgeUnit` = `ForgeOrg`); R5b adds units for
  Azure organizations and projects behind the same interface.
- **The claim stays on `dojo.tenants`** (`claimed_at`, `claimed_by` already existed): the claim gates billing and
  private sync for the whole dōjō (`all_my_repositories` reads it there), and a dōjō may have several connections.
  `dojo.claim_forge_unit(principal, provider, external_id)` — service_role only — re-proves the connection
  (`verified_at`), claims the tenant if unclaimed (first claim wins) and writes a `dojo_claimed` audit row. The
  sign-in claim (`claimTenantIfOwner`) now goes through it too, so there is one claim path.
- **`dojo.forge_unit_mappings`** `(provider, external_id unique, external_slug, tenant_id, workspace_id null,
  proven_at, mapped_by, mapped_at)`: where a unit's repositories file. No row = its own connected dōjō (what
  provisioning makes). `dojo.map_forge_unit` (service_role) enforces: the mapper administers the target dōjō; *its own
  dōjō* needs proof and must be the connected dōjō; *a workspace* needs proof only once the org's dōjō is claimed or
  an admin of the org placed it (`proven_at`) — so a member can file an unclaimed org into a dōjō they administer,
  and cannot take a claimed one away from its admins. Audited (`forge_unit_mapped`, and `forge_unit_unmapped` in the
  dōjō it left). RLS: the dōjō's admins read; nobody writes directly.
- **Registration** — `dojo.register_repository` consults the unit's mapping first: mapped, the repository files into
  the mapped dōjō (and workspace) for that dōjō's members, and `not_a_member` for anyone else — never into the org's
  own dōjō; unmapped, by connection as before. The daemon's own workspace does not re-file a mapped repository.
  Repositories already registered move on their next registration (the daemon re-registers every cycle).
- **Provisioning (decision)** — a sign-in does **not** create a dōjō for an org an admin mapped as a workspace of
  another dōjō: its repositories already file there. An org whose dōjō already exists keeps it, and #309's
  membership lifecycle is unchanged.
- **Screen** — org Admin → Integrations. The person pastes a GitHub token (read:org): the dōjō has no durable
  GitHub token — `provider_token` exists only during the OAuth exchange — so the proof is read with a token given
  for that request, used for its reads only, never stored, logged or echoed (as D35 does for Azure). Routes:
  `POST /v1/t/{tenant}/integrations/github` (list), `…/claim`, `…/map` (admin floor of the screen's dōjō).
  An org in a workspace of a dōjō the person is not in is shown without naming that dōjō.
- **Not in R5a**: removing a mapping (re-map instead), listing all dōjōs to pick a target (the target is the dōjō
  whose screen this is), a re-sign-in flow that would replace the pasted token, and membership sync from
  `listMembers` (provisioning still follows each person's own org list, #309).

## R5b · Azure DevOps (after R5a)

The same adapter and screen; organization → dōjō, project → workspace (D36); the PAT in Vault (D35).

## The PAT is a secret (D35's consequence)

- Stored in **Supabase Vault** (`vault.secrets`, encrypted at rest); `tenant_connections` keeps only the
  Vault secret's id. Reading it is a `SECURITY DEFINER` function executable by `service_role` only.
- **Never** returned to a client, never logged, never in an error message, never in an audit row (the
  audit row records "token set / replaced / removed" and who did it).
- Validated at paste time: the token must list the organization and prove the admin (Project Collection
  Administrators) before it is stored; a failing token is refused, not stored.
- Scope it needs: read access to the organization, its projects, members and code. The screen lists what it
  needs and warns when the token also carries write scopes.
- Rotation: an admin replaces it; expiry is read from the API when available and shown; an expired or
  revoked token marks the connection **needs a new token** and stops member reads (never ends a
  membership on a failed read — #309's completeness rule).

## Done gate

- An Azure DevOps organization admin imports their organization: one dōjō, one workspace per project,
  repositories filed by `repo-mapping`; a non-admin cannot create that dōjō.
- A complete member read ends a departed provisioned member; a partial read ends nobody.
- A GitHub org still provisions on sign-in exactly as today.
