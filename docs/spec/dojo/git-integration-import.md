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
- **Azure DevOps:** organizations via the Profiles and Accounts APIs, projects per organization, admin via the
  organization's Project Collection Administrators group. Auth: Microsoft Entra (D42).
- **GitLab, Bitbucket:** later (Q3).

## Screens

- **Admin → Integrations → Connect an organization:** pick a provider, sign in (GitHub re-authorization;
  Azure DevOps through Microsoft Entra, D42),
  see the units with a proposed mapping (dōjō / workspace of …), adjust, import. Units the person
  cannot prove are marked and can only become workspaces of a dōjō they already administer.
- **The personal dōjō** offers the same import for units the person owns.

## Decisions (user, 2026-10-22)

- **D35 · Azure DevOps sign-in is a personal access token** an admin pastes (no app registration).
  **Superseded by D42** (user, 2026-10-23): a PAT cannot list its owner's organizations, read its own scopes
  or report its expiry.
- **D36 · Azure maps organization → dōjō, project → workspace** by default.
- **D37 (revised, user 2026-10-22) · One complete source first.** **R5a** finishes the flow for GitHub end to
  end; **R5b** adds Azure DevOps on the same adapter and screen only after R5a is done and verified. GitLab
  and Bitbucket later.
- **D42 (user, 2026-10-23) · Azure DevOps connects through Microsoft Entra OAuth** — a delegated,
  authorization-code connect run by the dōjō server, separate from dōjō sign-in, with one multi-tenant app
  registration for sensei. The refresh token is kept in Vault; access tokens never are. MSA-backed
  organizations cannot connect (Microsoft limitation) and the screen says so.
- **D43 · Azure members match by stable id only**, never email.
- **D44 · The Azure member read also runs on a schedule.**
- **D45 · "Sign in with Microsoft" is R5c** (kavach's `azure` provider, Supabase Auth's Azure provider) on the
  SAME app registration; R5b keys Azure identities on the id R5c will also see.

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

The same adapter and screen; organization → dōjō, project → workspace (D36). The person connects through
**Microsoft Entra** (D42): a server-side, confidential-client authorization-code flow with PKCE against the
multi-tenant `organizations` authority, separate from dōjō sign-in (it never goes through Supabase Auth, so
connecting Azure never swaps the person's dōjō login — R5a's caveat). The dōjō keeps the **refresh token** in
Vault per connection; access tokens live for one request.

1. **Connect** — an org dōjō's admin (Admin → Integrations, the import connects to THAT dōjō) or anyone from
   their personal dōjō (My dōjōs, each import becomes its own dōjō). The person signs in at Microsoft with a
   **work or school account**; personal Microsoft accounts (MSA) cannot use the Azure DevOps resource through
   Entra ("Microsoft Entra apps don't natively support Microsoft account (MSA) users for the Azure DevOps
   resource" — [entra-oauth](https://learn.microsoft.com/en-us/azure/devops/integrate/get-started/authentication/entra-oauth)),
   and the screen says so.
2. **Scopes** — the Azure DevOps resource (`499b84ac-1321-427f-aa17-267ca6975798`) read scopes, written
   `499b84ac-1321-427f-aa17-267ca6975798/vso.profile`, `…/vso.project`, `…/vso.graph`,
   `…/vso.memberentitlementmanagement`, `…/vso.code`, plus `offline_access` (the refresh token) and `openid`
   ([available scopes](https://learn.microsoft.com/en-us/azure/devops/integrate/get-started/authentication/oauth#available-scopes)).
   `vso.profile` is needed by the Profiles and Accounts reads that list the person's organizations.
3. **Pick and import** — the organizations the grant can see are listed; each one imported is validated first
   (the person active in it, a Project Collection Administrator, its projects read) and only then written:
   connection + claim, the refresh token in Vault, each project a workspace, the members read (#309).
4. **Write-scope warning** — the scopes granted are read from the token response's `scope` field ("the scopes
   that the access_token is valid for",
   [auth code flow](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow)); any
   beyond the read set (e.g. `vso.code_write`, granted to the app earlier) is named on the screen. The access
   token's `scp` claim is NOT decoded: Microsoft says not to read tokens for APIs you don't own.
5. **Validity** — a refresh Entra answers `invalid_grant` (revoked, expired, consent withdrawn) marks the
   connection `needs_new_token`: "Reconnect Azure DevOps"; reads stop and no membership is ended (#309).
6. **Identity (D43/D45)** — an Azure identity is keyed on the person's **Microsoft Entra object id**: for the
   connecting admin, Azure DevOps Graph `Users - Get` on their subject descriptor (`origin` = `aad`,
   `originId` = the object id —
   [graph users](https://learn.microsoft.com/en-us/rest/api/azure/devops/graph/users/get?view=azure-devops-rest-7.1));
   for members, the user entitlement's `user.originId` when `user.origin` is `aad`. That is the id a Microsoft
   sign-in (R5c) also sees — R5c must match on the Entra `oid`, not `sub` (pairwise per application) — so R5c
   adds sign-in without re-keying identities. Never email (D43). Members who are not Entra users (e.g.
   GitHub-backed Azure DevOps users) can be matched by no identity and are not listed. Microsoft notes that
   linking/unlinking a user to another provider can change `originId`.
7. **Scheduled member read (D44)** — a Cloudflare Cron Trigger reads each working connection's members and
   applies #309's rule, bounded per run, recording each connection's outcome.

### R5b as built (2026-10-23, branch `r5b-azure-import`)

- **The grant in Vault** — `tenant_connections` keeps only `token_secret_id` (a `vault.secrets` id),
  `token_state` (`dojo.forge_token_state`: `ok | needs_new_token`; NULL = none) and `token_expires_at` (NULL:
  Entra does not report a refresh token's lifetime), plus `members_read_at` / `members_read_error` (the
  scheduled read's record). `dojo.set_forge_token`, `rotate_forge_token`, `remove_forge_token`,
  `forge_token` and `mark_forge_token_rejected` are SECURITY DEFINER, service_role only (anon, authenticated
  and PUBLIC revoked). Set/replace/remove need an ACTIVE admin of the dōjō and audit `forge_token_set |
  _replaced | _removed` (actor, provider, org name — never the token); a refused grant audits
  `forge_token_rejected`; rotation (each refresh) updates the one secret in place, unaudited. Rotate and
  reject are compare-and-set on the token the caller refreshed (a sync racing a reconnect never marks or
  overwrites the newer grant), and rotate never touches `token_state`. A deleted
  connection deletes its secret (trigger). `forge_token` raises rather than answering NULL (none → 404,
  rejected → 409). Pinned by `database/tests/dojo/forge_token_vault.sql`.
- **Entra client** — `dojo/src/lib/server/azure-entra.ts`: authorize URL (S256 PKCE, `prompt=select_account`),
  code exchange and refresh (client secret in the form body, never a URL; `invalid_grant` on refresh is
  `AzureGrantRejected`; other errors are 502 by error code only), granted scopes from the response, config from
  `AZURE_DEVOPS_CLIENT_ID` / `AZURE_DEVOPS_CLIENT_SECRET` (missing → 503 "Azure DevOps is not configured on this
  dōjō", never a fallback).
- **Connect flow** — `azure-connect.ts` on the shared `sealed-value.ts` primitive (R5a's AES-GCM/HKDF seal,
  generalised with a per-purpose key; the GitHub cookie keeps its purpose, so its cookies still open):
  - `POST /v1/t/{origin}/{org}/integrations/azure_devops/connect` (admin floor, organization origin) and
    `POST /v1/you/integrations/azure_devops/connect` answer `{ url }` and set `dojo_azure_state` — a random
    `state`, the PKCE verifier and the target, sealed for the login, 10 minutes, HttpOnly, SameSite=Lax,
    Path=/v1, Secure on https.
  - `GET /v1/integrations/azure_devops/callback` (the registered redirect URI) identifies the person by their
    dōjō session cookie; the state must open for that login and match, single-use, before the code is redeemed;
    the refresh token is sealed as `dojo_azure_grant` (15 minutes). Every outcome is a 303 back to the screen:
    `?azure=connected` or `?azure_error=expired|state|denied|entra|signin|not_configured|too_many`.
  - `POST /v1/integrations/azure_devops/organizations` lists what the grant sees (`listAzureOrganizations`:
    Profiles `me` → Accounts by member id), with the write scopes and the target; `POST …/import
    {organization}` imports one where the connect started (an org dōjō target re-checks the admin). Using the
    grant refreshes it and re-seals the rotated token. 428 without a grant.
  - Every connect step is rate-limited: `dojo.request_attempts` (#311's limiter, generalised: `invite_accept`
    and `azure_connect`, 30 per 15 minutes) — `rate-limit.ts`. An attempt is taken atomically by
    `dojo.take_attempt` (count + record under an advisory lock per person and action, the action's old rows
    purged); `dojo.release_attempt` gives one back (a successful invite accept). The callback validates the state
    first: only a state that opens for the login and matches is counted, spent and redeemed.
- **Adapter** — `forge-azure.ts`, `azureAdapter(org)` behind `ForgeAdapter` plus `connection`, `owner` and
  `listProjects`. Bearer reads only to `app.vssps.visualstudio.com`, `dev.azure.com`, `vssps.dev.azure.com`,
  `vsaex.dev.azure.com`, https, redirects not followed (203/302 read as refused). Org names validated before
  any URL; descriptors and continuation tokens validated or encoded. `proveAdmin`: exactly one
  `[org]\Project Collection Administrators` group whose domain is the collection scope
  `vstfs:///Framework/IdentityDomain/{org id}` (a project group of that name is refused), then the owner's
  memberships walked up; false only after a finished walk, throws otherwise. `listMembers`: user entitlements
  (`members` or `items`), continuation token as a string or one-string array (anything else throws), keyed by
  Entra object id; incomplete on an Entra entry with no object id. The org's stable id and the owner's
  descriptor come from `GET https://dev.azure.com/{org}/_apis/connectionData` (Microsoft's own client's connect
  read, `azure-devops-node-api` `ConnectionData`; not in the REST reference).
- **Import / sync** — `azure-import.ts`: validate, then (org dōjō target) connect to THAT dōjō (409 if another
  holds it) and claim via `claim_forge_unit`, or (personal) make or join the org's own dōjō via
  `ensureOrgTenant`; record the owner's identity (`dojo.auth_method` `azure_devops`, subject = Entra object id);
  store the refresh token; map each project (`map_forge_unit`, stable id, `{org}/{project}`, `p_keep_elsewhere`
  so a project another dōjō holds stays there — enforced in the database); follow the members. Sync refreshes
  the stored grant (`rotate_forge_token`), re-proves, files, follows.
- **Repositories file by project** — `repo-mapping.ts` names the project; `register_repository(…, p_project)`
  resolves the project's mapping, then the org's, then the connection; `dojo.repo_key_names` checks the org and
  project AT THEIR PLACES in the key (percent-decoded, lowercased); a key over 2 KB is refused; mapping
  ambiguity counts distinct places; `map_forge_unit` serialises maps of one unit with an advisory lock.
  `database/tests/dojo/azure_project_filing.sql`, `register_repository_access.sql`.
- **Membership** — `memberships.source_provider` / `source_unit` record which forge and unit provisioned a row.
  `dojo.reconcile_unit_members(tenant, provider, unit, auth, subjects)` ends/restores only rows that unit
  provisioned (`left_org`); `reconcile_org_memberships` only GitHub's. Provenance is attributed from the code,
  never guessed (`apply/after/membership_source_backfill.sql`): only `ensureMembership` writes `provisioned`, in
  an org dōjō only with `github_oauth` (GitHub) or `azure_devops` (the import); `sso` and `device_code` org rows
  and personal-dōjō rows stay unattributed, are reported, and no reconcile acts on them. #309's relabel applies
  only to rows from before v0.16.0. A NULL/empty member list is refused. GitHub sign-in provisioning is
  unchanged. `unit_member_departure.sql`, `membership_departure.sql`.
- **Scheduled read** — `azure-member-sync.ts`: up to 10 working connections per run, longest-unread first;
  refresh → the same admin proof Sync and import run (`proveGrant`: active, Project Collection Administrator) →
  same org? → members → reconcile; each outcome recorded on the connection and logged, one failure (or one
  unrecordable outcome) never stops the others, and the run fails at the end if any could not be recorded.
  `DOJO_CRON_SECRET` must be at least 32 characters. Trigger: `wrangler.jsonc` `triggers.crons ["17 */6 * * *"]`. adapter-cloudflare 7.x
  has no `scheduled` export and writes its worker to its config's `main`, so it builds against
  `wrangler.sveltekit.jsonc` and the deployed `main` is `dojo/worker-entry.js`, which re-exports `fetch` and adds
  `scheduled()`: an in-process `POST /v1/internal/azure/member-sync` bearing `DOJO_CRON_SECRET` (constant-time
  check; 401 otherwise, 503 when unset — never open). A failed run throws (Cron Events).
- **Screen** — `AzureConnect` (kit): Connect (no token field anywhere), the read scopes, the work-or-school
  note, the callback's outcome, the organizations to import with the write-scope warning, and on an org dōjō each
  connection's state with Sync and Remove. On Admin → Integrations beside GitHub, and on My dōjōs.
- **Not in R5b**: Microsoft sign-in (R5c, D45 — see "R5c as built"); provisioning Azure members who are not yet dōjō users (only
  people who connected hold an Azure identity, so only they can be provisioned or ended by a member read).

### R5b · one-time setup (the user's checklist)

One **multi-tenant app registration** serves both R5b's connect and R5c's Microsoft sign-in (D45).

1. **Create it** — Microsoft Entra admin center → App registrations → New registration. Name: `sensei dōjō`.
   Supported account types: **Accounts in any organizational directory (Any Microsoft Entra ID tenant —
   Multitenant)**. Personal Microsoft accounts are not offered: MSA cannot use the Azure DevOps resource.
2. **Redirect URIs** — platform **Web** (not SPA), all four:
   - R5b connect, local: `http://localhost:5173/v1/integrations/azure_devops/callback` (Microsoft accepts
     plain http only for `localhost`, not `127.0.0.1`; locally, use the dōjō at `http://localhost:5173`
     throughout — its session cookie is per host).
   - R5b connect, production: `https://<dōjō origin>/v1/integrations/azure_devops/callback`.
   - R5c sign-in (Supabase), local: `http://localhost:54321/auth/v1/callback`.
   - R5c sign-in (Supabase), production: `https://<project-ref>.supabase.co/auth/v1/callback`.
3. **API permissions** (delegated) — Microsoft Graph: `openid`, `email`, `profile` (R5c sign-in asks exactly
   these) and `offline_access` (R5b connect's refresh token). Azure DevOps: `vso.profile`, `vso.project`,
   `vso.graph`, `vso.memberentitlementmanagement`, `vso.code` (R5b connect) — read scopes only; do not add
   `user_impersonation` or any `_write`/`_manage` scope. Tenants that restrict user consent need their Entra
   admin to grant consent to the app.
4. **Token configuration** — add optional claims `email` and `xms_edov` to the ID token (Supabase's guidance
   for a verified email, R5c).
5. **Certificates & secrets** — one client secret (note its expiry; rotate before it). Used in two places:
   - the dōjō's connect: `AZURE_DEVOPS_CLIENT_ID` and `AZURE_DEVOPS_CLIENT_SECRET` — locally in `dojo/.env`
     (Vite) and `dojo/.dev.vars` (wrangler); in production as Cloudflare **Worker secrets**
     (`wrangler secret put AZURE_DEVOPS_CLIENT_ID`, `… AZURE_DEVOPS_CLIENT_SECRET`);
   - Supabase's Azure provider (R5c): locally `supabase/config.toml` `[auth.external.azure]` (client id, the
     secret through `env(…)`), in production the Supabase dashboard.
6. **Scheduled member read** — a Worker secret `DOJO_CRON_SECRET` (a long random string); locally in
   `dojo/.dev.vars` for `wrangler dev --test-scheduled`.
7. **Verify** — with an Azure DevOps organization in which the person is a Project Collection Administrator:
   Connect from My dōjōs → Microsoft → back with `?azure=connected` → the organization listed → Import → a new
   dōjō with one workspace per project; then Admin → Integrations → Sync; a repository under
   `dev.azure.com/<org>/<project>/_git/<repo>` registered by the daemon files into that project's workspace.
   Check that `vault.secrets` holds one secret per connection and `audit_events` names `forge_token_set`
   without a token.

### R5c as built (2026-10-23, branch `r5c-microsoft-signin`)

- **The sign-in page** — `kavach.config.js` lists `{ mode: 'oauth', name: 'azure', label: 'Continue with
  Microsoft', scopes: ['email', 'profile'] }` right after GitHub. GoTrue always asks `openid` first and appends
  the client's scopes (`supabase/auth` `internal/api/provider/azure.go`, `oauthScopes := []string{"openid"}`),
  so the request is exactly `openid email profile`. `DojoSignIn` renders every configured provider (the vitest
  stub is now the config's own list), and `uno.config.js` aliases `i-auth-azure` to the Microsoft logo.
- **Nothing GitHub-only runs on a Microsoft token** — after a Microsoft sign-in the session's `provider_token`
  is a Microsoft Graph token. `isGithubToken` (`github-identity.ts`) recognises GitHub's documented formats
  (`gho_`, `ghu_`, `ghp_`, `ghs_`, `ghr_`, `github_pat_`); `provisionWithToken` treats any other token as no
  forge token, and `captureForgeToken` neither sends it to GitHub nor seals it. Before this, a login linked to
  GitHub and Microsoft would have posted its Microsoft token to `api.github.com` and read the 401 as a dead
  GitHub grant.
- **Local Supabase** — `supabase/config.toml` `[auth.external.azure]`: enabled, `client_id` /
  `secret` from `SUPABASE_AUTH_EXTERNAL_AZURE_CLIENT_ID` / `_SECRET` (named in `dojo/.env.example`), `url =
  "https://login.microsoftonline.com/organizations"`, `redirect_uri = "http://localhost:54321/auth/v1/callback"`;
  `additional_redirect_urls` gains `http://localhost:5173`, `http://localhost:5173/**`, `http://localhost:5175/**`.
  Like GitHub's block, unset variables only warn: verified on a throwaway stack (its own project id and ports)
  that `supabase start` succeeds with both unset, `/auth/v1/settings` lists `azure`, and
  `/auth/v1/authorize?provider=azure&scopes=email%20profile` redirects to
  `login.microsoftonline.com/organizations/oauth2/v2.0/authorize` with `scope=openid+email+profile` and the
  localhost `redirect_uri`. Locally, use the dōjō at `http://localhost:5173`. The running local Supabase needs a
  restart (`supabase stop && supabase start`, data kept) to pick the block up.
- **Identity** — `azure-signin.ts` `microsoftIdentity(user)` reads the login's Microsoft account from the
  Supabase user (getUser, never a request's claim): `identities[provider = azure].identity_data.custom_claims.oid`
  (where GoTrue keeps it: see the risk below). Never `sub` (GoTrue's `provider_id`, pairwise per app), never
  email (D43), never `user_metadata` (the person can rewrite it). The oid must be a GUID and is lowercased through
  `entraObjectId` (forge-azure.ts), the normalisation R5b's import and member read apply to `originId`. No oid, a malformed one, or two Microsoft accounts on one login is
  `unlinkable`.
- **Member sets** — `dojo.connection_members (connection_id, subject, read_at)`, owned by the connection
  (deleted with it), service_role only: the connection's last COMPLETE member set. `dojo.reconcile_unit_members`
  (only a complete read calls it: import, Sync, the scheduled read) now also replaces the set and provisions
  every listed person who already holds the identity but has no membership in the dōjō — `contributor` (a
  member read proves no admin; the forge-read rule `roleForOrg` applies), `employer`, provenance the forge and
  unit. An existing membership always wins. It returns `(ended, joined)`;
  `apply/before/dojo_reconcile_unit_members_retyped.sql` drops the integer version first. A partial, empty or
  failed read never calls it, and a NULL/empty list is refused with the set unchanged.
  `dojo.units_listing_member(provider, subject)` names the organization dōjōs whose last complete set lists a
  person (personal dōjōs never). Pinned by `database/tests/dojo/connection_member_sets.sql`.
- **Provisioning on sign-in** — `ensureProvisioned` (shared by the kavach session hook, the CLI token
  exchange and `POST /v1/you/provision`, all of which pass `microsoft`) records the oid as the person's
  `azure_devops` identity through `ensureForgeIdentity` (another person's: 409, never a re-point) and joins each
  dōjō `units_listing_member` names through `ensureMembership`, `from = { azure_devops, unit }` — so that unit's
  next complete read ends it when they leave (#309). The answer carries `microsoft: { linked: true }`.
  `unlinkable` FAILS CLOSED: no identity, no provisioning, `console.error` with the reason, and
  `microsoft: { linked: false, reason }`. The personal dōjō and the GitHub half are unchanged.
- **Visible state** — `GET /v1/you/microsoft` (`microsoftLinkStatus`, computed, never stored): `none`, `linked`,
  or `not_linked` with the reason (no object id; several Microsoft accounts; held by another dōjō login; not
  recorded). My dōjōs shows "Couldn't link your Microsoft account: <reason>. Your Azure DevOps organizations'
  dōjōs were not joined." as an alert; a failed check is said, never read as linked.
- **Not in R5c**: provisioning a Microsoft user into an organization no complete read has listed them in yet
  (they join at the next import, Sync or scheduled read, which provisions listed people who hold the identity);
  B2B guests (an Azure DevOps organization lists a guest by the object id in ITS tenant, while sign-in through
  `organizations` yields their HOME tenant's object id, so they do not match).

**RISK — where the object id comes from (unverified offline).**
- *Assumed*: Supabase Auth keeps the Entra `oid` claim at `identities[provider = azure].identity_data.custom_claims.oid`.
  Source: `supabase/auth` `internal/api/provider/oidc.go` `parseAzureIDToken` copies every verified ID-token claim
  except `aud iss iat nbf exp c_hash at_hash aio nonce rh uti jti ver sub name preferred_username` into
  `Metadata.CustomClaims`, serialised as `custom_claims` (`provider.go` `Claims`); `external.go` stores
  `structs.Map(userData.Metadata)` as the identity's `identity_data` and rewrites it at every sign-in. Microsoft
  puts `oid` in every v2.0 ID token ([ID token claims](https://learn.microsoft.com/en-us/entra/identity-platform/id-token-claims-reference)).
  Read at `supabase/auth` v2.197.0; the running local stack is GoTrue v2.193.0.
- *Why unverified*: it needs a real Microsoft sign-in, which needs the app registration; there is none yet.
- *If it is wrong*: every Microsoft sign-in is `unlinkable` — it fails closed: no identity, no membership, a
  logged reason, and the My dōjōs alert. Nothing keys on `sub` or email.
- *The live test that settles it*: sign in with Microsoft as a member of a connected Entra-backed Azure DevOps
  organization; then compare `select subject from dojo.identities where provider = 'azure_devops' and principal_id
  = <their principal>` with that person's `user.originId` in the organization's user entitlements (Azure DevOps
  `GET https://vsaex.dev.azure.com/{org}/_apis/userentitlements`, `user.origin = aad`). Equal: settled. My dōjōs
  shows the alert instead: the assumption is wrong, and the identity data GoTrue recorded
  (`auth.identities.identity_data` for that user) says where the claim went.

### R5c · what the user sets

On the **same** app registration as R5b (its one-time setup above):
1. **Redirect URIs** (platform Web) — the two Supabase callbacks: `http://localhost:54321/auth/v1/callback`
   (local) and `https://<project-ref>.supabase.co/auth/v1/callback` (production; the dashboard's Azure provider
   shows it).
2. **API permissions** — Microsoft Graph delegated `openid`, `email`, `profile`.
3. **Token configuration** — optional ID-token claims `email` and `xms_edov`. Supabase's Azure provider needs an
   email (without one GoTrue refuses the sign-in: "Error getting user email from external provider"). With
   `xms_edov` in the token, GoTrue counts the email verified only when the domain owner verified it; WITHOUT the
   claim it assumes verified (`AzureIDTokenClaims.IsEmailVerified`). That matters because Supabase links a new
   identity to an existing login by verified email: without `xms_edov`, a Microsoft account carrying someone
   else's unverified address could join that person's login.
4. **Production Supabase** — Dashboard → Authentication → Sign In / Providers → **Azure**: enabled; Application
   (client) ID = the app registration's client id; Secret Value = one of its client secrets; Azure Tenant URL =
   `https://login.microsoftonline.com/organizations`. The production dōjō origin is already among the redirect
   URLs (GitHub's sign-in uses it).
5. **Local** — export `SUPABASE_AUTH_EXTERNAL_AZURE_CLIENT_ID` and `SUPABASE_AUTH_EXTERNAL_AZURE_SECRET` in the
   shell that runs `supabase start` (then restart the local stack) and open the dōjō at `http://localhost:5173`.
6. **Database** — the release's `dbd` converge adds `dojo.connection_members` and `dojo.units_listing_member`
   and retypes `dojo.reconcile_unit_members` (the before-hook drops the old one). Member sets fill on each
   connection's next import, Sync or scheduled read; until then a Microsoft sign-in joins nothing.

### R5c · live check

1. With the setup above, a member of a connected Entra-backed Azure DevOps organization (not a guest) whose
   organization has had a complete member read since the deploy: **Continue with Microsoft** → lands on the dōjō;
   My dōjōs lists that organization's dōjō, as contributor; no alert.
2. The risk test above: their `dojo.identities` `azure_devops` subject equals their entitlement's `originId`.
3. In the organization, remove them; run Sync (or wait for the scheduled read): their membership ends
   (`left_org`), and `dojo.connection_members` no longer lists them.
4. A person whose Microsoft account is also linked to GitHub signs in with Microsoft: no request reaches
   `api.github.com` with the Microsoft token (the GitHub sync answers `no_forge_token`, not
   `forge_token_rejected`).

## The Azure grant is a secret (D42's consequence)

- Stored in **Supabase Vault** (`vault.secrets`, encrypted at rest): the connecting admin's Entra **refresh
  token**, rotated in place on every refresh. Access tokens are never stored. `tenant_connections` keeps only
  the Vault secret's id; reading it is a `SECURITY DEFINER` function executable by `service_role` only.
- **Never** returned to a client, never logged, never in an error message, never in an audit row (the audit row
  records "token set / replaced / removed" and who did it). Between the callback and an import it lives only in
  a sealed, HttpOnly cookie bound to the person's login (15 minutes).
- Validated before it is stored: the grant must reach the organization and prove the admin (Project Collection
  Administrators); a failing grant is refused, not stored.
- Scopes: read access to the profile, the organization's projects, graph, members and code. The screen lists
  them and warns when the consent also carries more.
- Rotation and loss: Entra rotates the refresh token on each use; a refused refresh (`invalid_grant`) marks the
  connection **needs a new token** ("Reconnect Azure DevOps") and stops member reads — never ending a membership
  on a failed read (#309). An admin can remove the stored grant (audited).

## Done gate

- An Azure DevOps organization admin imports their organization: one dōjō, one workspace per project,
  repositories filed by `repo-mapping`; a non-admin cannot create that dōjō.
- A complete member read ends a departed provisioned member; a partial read ends nobody.
- A GitHub org still provisions on sign-in exactly as today.
