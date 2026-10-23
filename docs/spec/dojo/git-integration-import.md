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
- **Screen** — org Admin → Integrations, with **Connect GitHub** (user decision 2026-10-22: re-authorization, no
  pasted token). The button runs the dōjō's own GitHub sign-in again through kavach (`kavach.signIn` with the
  configured `github` provider and its scopes — `read:org` among them; the same scopes as sign-in, because the
  returning session also runs provisioning, whose repository-role mirror clears roles a narrower token cannot list)
  and returns to the screen. How the token travels:
  1. GitHub → Supabase → the browser (the existing OAuth redirect); the browser's kavach instance POSTs the session,
     `provider_token` included, to `/auth/session` — exactly as every sign-in already does.
  2. `hooks.server.ts` wraps kavach's answer to that POST (`captureForgeToken`). It reads GitHub's `GET /user` with
     the posted token and continues only if the id equals the GitHub account Supabase Auth linked to the synced
     login (`user.identities[provider=github].id`, `github-identity.ts`) — a token of any other account, or one
     GitHub will not identify, is never sealed (security review: otherwise a person holding someone else's token
     could claim that account's orgs as theirs). Then it adds a cookie `dojo_forge_github` holding the token **sealed** — AES-GCM under a key HKDF-derived from the server's
     secret key, bound to the Supabase login the session synced as and to an expiry — `HttpOnly; SameSite=Lax;
     Path=/v1/t; Max-Age=3600`. The database never holds it; signing out clears it.
  3. Each Integrations request (`POST /v1/t/{tenant}/integrations/github`, `…/claim`, `…/map`; admin floor) opens
     the cookie for the caller's own login (`integrations-route.ts`) and uses the token for that request's reads
     only. No cookie, one that does not open (another login, expired, tampered) or a token GitHub answers 401 to:
     **428** "Connect GitHub" (and the dead cookie is cleared) — there is no other source of the token.
  The request bodies carry no token. The ownership check runs once, at seal time: the cookie is authenticated
  encryption under the server's key and bound to the login, so a cookie can only exist for a token already proven
  to be that login's GitHub account; re-reading `GET /user` on every request would add a GitHub round trip and
  prove nothing new. The same check guards provisioning (`provisionWithToken`, used by the sign-in hook, the CLI
  token exchange and `POST /v1/you/provision`): the `GET /user` it already makes must be the login's linked GitHub
  account, or it refuses with 403 before any write; the sign-in hook now also takes the login from Supabase's
  validation of the access token, not from the session's own `user` claim. Lifetime: at most one hour, or until sign-out. Supabase must allow the return
  URL (`<origin>/org/**` in the redirect allow-list — added to `supabase/config.toml` for local; production's is a
  dashboard setting); without it Supabase returns to the site URL, the cookie is still set, and the screen works
  once reopened. Caveat: re-authorizing a magic-link session with GitHub signs in as the GitHub-linked Supabase
  user, which is a different login if the emails differ.
  An org in a workspace of a dōjō the person is not in is shown without naming that dōjō.
- **Not in R5a**: removing a mapping (re-map instead), listing all dōjōs to pick a target (the target is the dōjō
  whose screen this is), and membership sync from
  `listMembers` (provisioning still follows each person's own org list, #309).

## R5b · Azure DevOps (after R5a)

The same adapter and screen; organization → dōjō, project → workspace (D36); the PAT in Vault (D35).

**Deviations found while building (for the user to confirm, 2026-10-23):**
- **No organization list from a PAT.** "Organizations via the accounts API" does not work with the token D35 chose:
  Microsoft's PAT page says the Organizations and Profiles APIs need a Microsoft Entra token, an organization-scoped
  PAT gets an empty accounts list, and global ("all accessible organizations") PATs stop working on **1 Dec 2026**
  ([PAT docs](https://learn.microsoft.com/en-us/azure/devops/organizations/accounts/use-personal-access-tokens-to-authenticate)).
  So the admin types the organization's URL beside the token, and the adapter is bound to that one organization:
  `listUnits` answers whether the token's owner is active in it (with their role proven), not "every organization".
- **The token owner and the organization's stable id come from `GET https://dev.azure.com/{org}/_apis/connectionData`**
  (`authenticatedUser.id`/`subjectDescriptor`/`isActive`, `instanceId`). It is not in the REST reference; it is the
  call Microsoft's own client makes on connect (`azure-devops-node-api`, `LocationsInterfaces.ConnectionData`). The
  documented alternative, the Profile API, needs an Entra token.
- **No write-scope warning.** A PAT cannot read its own scopes: the PAT Lifecycle Management API takes only a
  Microsoft Entra token. The screen therefore lists the four read-only scopes to give the token and asks for a
  read-only, single-organization token; it does not claim to check.
- **Expiry is unknown.** For the same reason Azure does not tell a PAT its expiry; `token_expires_at` stays NULL
  and the screen says "expiry not reported by Azure DevOps". A rejected token (401) is what marks it
  `needs_new_token`.

### R5b as built (2026-10-23, branch `r5b-azure-import`)

- **The PAT in Vault** — `tenant_connections` gained `token_secret_id` (a `vault.secrets` id), `token_state`
  (`dojo.forge_token_state`: `ok | needs_new_token`; NULL = none) and `token_expires_at`. `dojo.set_forge_token`,
  `remove_forge_token`, `forge_token` and `mark_forge_token_rejected` are SECURITY DEFINER, service_role only (anon,
  authenticated and PUBLIC revoked). Writes need an ACTIVE admin of the connection's dōjō and audit
  `forge_token_set | forge_token_replaced | forge_token_removed` (actor, provider, org name — never the token);
  a 401 audits `forge_token_rejected`. A replace updates the one secret in place (`vault.update_secret`); a removed
  or deleted connection deletes its secret (trigger `tenant_connections_forget_token`). `forge_token` raises rather
  than answering NULL: none stored → 404, rejected → 409. Pinned by `database/tests/dojo/forge_token_vault.sql`.
- **Adapter** — `dojo/src/lib/server/forge-azure.ts`, `azureAdapter(org, fetch)` behind `ForgeAdapter` plus
  `connection` and `listProjects`. PAT as Basic auth; only `dev.azure.com`, `vssps.dev.azure.com` and
  `vsaex.dev.azure.com`, https, redirects not followed (Azure answers a bad token with a sign-in page, 203/302 —
  read as a rejected token). Org names are validated (`[a-z0-9][a-z0-9-]{0,49}`) before any URL; descriptors and
  continuation tokens from Azure are validated or encoded. `proveAdmin`: the org's `Project Collection
  Administrators` group (Graph groups, paged by `X-MS-ContinuationToken`), then the token owner's memberships walked
  up through nested groups (Graph memberships, `direction=up`); true only for the org's stable id, false only after a
  finished walk, throws on a failed read, a missing group or more than 100 lookups. `listMembers`: user entitlements
  by `continuationToken` (the reference names the page `items`, the service answers `members`; both are read),
  removed/disabled users excluded, incomplete on an unnamed entry, throws on a failed page, a repeated token or more
  than 200 pages. `listProjects`: `$top/$skip`, by stable id. `forgeAdapter('azure_devops')` stays 501 — the adapter
  needs its organization.
- **Import** (`azure-import.ts`) — validate first (owner active in the org, a Project Collection Administrator, the
  projects read), then write. From an **organization dōjō's** Admin → Integrations (`POST
  /v1/t/{origin}/{org}/integrations/azure_devops/connect`): the org connects to THAT dōjō (409 if another holds it) —
  one dōjō known to GitHub and Azure — and is claimed via `claim_forge_unit`. From the **personal dōjō** (My dōjōs,
  `POST /v1/you/integrations/azure_devops/connect`): the org's own dōjō is made, or joined as a proven admin, through
  `ensureOrgTenant` — the only way an Azure org's dōjō is first made; no other create path was added. Then the token
  goes to Vault and each project is mapped (`map_forge_unit`, keyed by the project's stable id, named
  `{org}/{project}`) to a workspace of that dōjō; a project already filed there keeps its workspace across a rename,
  one filed elsewhere is left, slugs never repeat. The token owner's Azure identity is recorded
  (`dojo.auth_method` gained `azure_devops_pat`; one Azure account is one person — a 409 otherwise). `GET …/azure_devops`
  lists the dōjō's Azure orgs and token state; `POST …/sync` re-reads with the stored token (a 401 marks
  `needs_new_token`, 409); `DELETE …/token` removes it. A token Azure will not take is a 422 naming no token; an
  outage stays 502. Admin floor on every tenant route.
- **Repositories file by project** — `repo-mapping.ts` names the project (https, SSH v3, `{org}.visualstudio.com`
  with or without `DefaultCollection`, `_git/{repo}` named after its project, `%20` decoded); `register_repository`
  takes `p_project` and resolves the project's mapping first, then the org's, then the connection. The key must carry
  the project it names. Unmapped stays unmapped. `database/tests/dojo/azure_project_filing.sql`.
- **Membership** — `dojo.reconcile_unit_members(tenant, provider, auth, subjects)`: #309's rule read from the
  organization's side (an admin's stored token lists the org's members, where GitHub reads each person's own orgs).
  A complete, non-empty read ends the provisioned memberships of people holding an Azure identity none of which is
  listed (`left_org`) and restores those listed again; invited members, people with no Azure identity, personal dōjōs
  and dōjōs not connected to Azure are untouched; a NULL/empty list is refused. Import and sync call it only after a
  complete read; partial/empty/failed reads end nobody and say so. `database/tests/dojo/unit_member_departure.sql`.
  GitHub provisioning on sign-in is unchanged.
- **Screen** — `AzureConnect` (kit): organization URL, PAT in a `type=password autocomplete=off` field emptied after
  every request, the four read-only scopes, the result; on an org dōjō the connected orgs with token state, Sync and
  Remove token.
- **Not in R5b**: a scheduled member sync (it runs on import and on Sync); provisioning Azure members who are not yet
  dōjō users (there is no Azure sign-in — only people who imported with their own PAT hold an Azure identity, so
  only they can be ended by a member read); the token's expiry.

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
