# Dōjō auth & provisioning — Gherkin scenarios

> **Scope:** GitHub OAuth sign-in → tenant provisioning → email-linking →
> email-removal disconnection. Covers the `dojo.*` schema tables: `tenants`,
> `memberships`, `identities`, `projects`. Maps to the existing enums
> `tenant_origin` (`github|org`), `membership_kind`
> (`employer|client|community|personal`), `member_role`
> (`contributor|maintainer|lead|admin`), `auth_method` (`sso|github_oauth|
> device_code`).
>
> **Convention:** auto-provisioned tenants and memberships are **inactive**
> (`disabled_at` set, `sync_status = 'authenticating'`) until the user
> explicitly activates them. Inactive rows are invisible in the user's
> "My Dōjōs" view and do not participate in routing or governance.

---

## Feature: GitHub OAuth sign-in and initial provisioning

### Background

```gherkin
Given the Dōjō service is running
And Supabase auth is configured with a GitHub OAuth provider
And the user has a GitHub account
```

---

### Scenario 1: First-time sign-in 

```gherkin
Given the user has no existing identity in the Dōjō
 When the user signs in using GitHub
  And the GitHub API returns the user's primary email, verified emails, avatar, orgs, repositories
 Then a Supabase auth user is created (or matched) with the GitHub access token
 And a personal tenant is created
  | field    | value                                  |
  | key      | personal/<github_username>             |
  | origin   | org                                    |
  | org      | <github_username>                      |
  | name     | <github_display_name>'s Dōjō          |
  | scope    | private                                |
  | dojo_url | <dōjō_service_url>                     |
And a `dojo.memberships` row is created for the personal tenant:
  | field           | value          |
  | role            | admin          |
  | kind            | personal       |
  | authenticated_via | github_oauth |
  | disabled_at     | <now>          |
  | sync_status     | authenticating |
And the user's session is established with the Supabase JWT
```

---

### Scenario 2: First-time sign-in — org tenants auto-created from GitHub orgs

```gherkin
Given the user belongs to GitHub organizations:
  | org              | membership_role | repo_access |
  | acme-corp        | owner           | admin       |
  | open-source-org  | member          | read        |
  | side-project-org | maintainer      | write       |
When the user signs in using GitHub for the first time
Then a tenant is created for each GitHub org:
  | key                    | origin | org              | name                   | scope   |
  | github/acme-corp       | github | acme-corp        | acme-corp Dōjō        | private |
  | github/open-source-org | github | open-source-org  | open-source-org Dōjō  | private |
  | github/side-project-org| github | side-project-org | side-project-org Dōjō | private |
And a `dojo.memberships` row is created for each tenant:
  | tenant_key               | role        | kind     | disabled_at | sync_status    |
  | github/acme-corp         | admin       | employer | <now>       | authenticating |
  | github/open-source-org   | contributor | community| <now>       | authenticating |
  | github/side-project-org  | maintainer  | community| <now>       | authenticating |
And the role is derived from the GitHub org membership:
  | github_role | dojo_role   |
  | owner       | admin       |
  | member      | contributor |
  | maintainer  | maintainer  |
And all auto-created memberships are inactive (`disabled_at` is set)
And the personal tenant membership is also inactive
```

---

### Scenario 3: Auto-created tenants are inactive by default

```gherkin
Given the user has just signed in for the first time
And auto-created tenants exist for the user's GitHub orgs
Then all auto-created tenants have no active memberships
And the user sees only their personal tenant in "My Dōjōs" (if any active membership exists)
And auto-created org tenants are not visible in the user's console until activated
And no projects are routed to inactive tenants
```

---

### Scenario 4: Subsequent sign-in — existing user matched

```gherkin
Given the user has previously signed in and has an existing `dojo.identities` row
When the user signs in using GitHub again
Then the existing identity is matched by `(provider=github_oauth, subject=<github_user_id>)`
And the identity's `last_login_at` is updated
And the user's session is established with the Supabase JWT
And no duplicate identity or tenant rows are created
```

---

### Scenario 5: GitHub org membership changes — new org detected

```gherkin
Given the user previously signed in with GitHub orgs: [acme-corp, open-source-org]
When the user signs in again
And GitHub now reports an additional org: new-client-org (role: member)
Then a new tenant is created:
  | key                   | origin | org            |
  | github/new-client-org | github | new-client-org |
And a new membership is created:
  | tenant_key            | role        | kind    | disabled_at | sync_status    |
  | github/new-client-org | contributor | client  | <now>       | authenticating |
And existing tenants and memberships are unchanged
```

---

## Feature: Email alias linking

### Background

```gherkin
Given the user is authenticated via GitHub
And the GitHub API provides a list of verified emails
```

---

### Scenario 6: GitHub emails match an existing user by alias email

```gherkin
Given another user exists with email `work@acme-corp.com` (via a different auth provider)
And the GitHub user's verified emails include `work@acme-corp.com`
When the GitHub user signs in
Then the system detects the email match
And the existing user's `dojo.identities` is linked to the GitHub identity
And the GitHub identity's `user_id` is set to the existing user's `user_id`
And no duplicate Supabase auth user is created
And the existing user's memberships now include the GitHub org tenants
```

---

### Scenario 7: GitHub emails match the user's own primary email

```gherkin
Given the user's Supabase auth email is `personal@gmail.com`
And the GitHub verified emails include `personal@gmail.com`
When the user signs in
Then the email is confirmed as belonging to the same user
And no cross-user linking occurs
And the identity is updated with the full list of verified emails
```

---

## Feature: Email-to-org association via domain matching

### Background

```gherkin
Given the user has multiple verified emails from GitHub:
  | email              | verified |
  | personal@gmail.com | true     |
  | work@acme-corp.com | true     |
  | freelance@client.com | true   |
And tenants exist for the user's GitHub orgs
```

---

### Scenario 8: Email domain matches an org tenant

```gherkin
Given a tenant exists:
  | key              | origin | org       |
  | github/acme-corp | github | acme-corp |
And the user has a verified email `work@acme-corp.com`
When the system processes email-org association
Then the email `work@acme-corp.com` is associated with tenant `github/acme-corp`
And the association is recorded for routing and attribution purposes
And the tenant's `org_slugs` includes the GitHub org name
```

---

### Scenario 9: No email domain matches — no association

```gherkin
Given the user has a verified email `personal@gmail.com`
And no tenant has a domain matching `gmail.com`
When the system processes email-org association
Then no email-org association is created for `personal@gmail.com`
And the personal tenant remains associated with the user's personal scope
```

---

## Feature: GitHub email removal — disconnection

### Background

```gherkin
Given the user is authenticated via GitHub
And the user has verified emails and associated tenants
```

---

### Scenario 10: GitHub email removed — membership disconnected

```gherkin
Given the user's verified emails include `work@acme-corp.com`
And a tenant `github/acme-corp` exists with an active membership
When the user signs in again
And GitHub reports that `work@acme-corp.com` is no longer in the verified emails
Then the email `work@acme-corp.com` is marked as removed from the identity
And the membership for tenant `github/acme-corp` is disabled:
  | field       | value          |
  | disabled_at | <now>          |
  | sync_status | authenticating |
And the tenant itself is NOT deleted (other members may still belong)
And projects previously routed to this membership are reclassified as personal
And the user sees the tenant as inactive in "My Dōjōs"
```

---

### Scenario 11: GitHub email removed — multiple tenants affected

```gherkin
Given the user's verified emails include:
  | email              | associated_tenant |
  | work@acme-corp.com | github/acme-corp  |
  | freelance@client.com | github/client-org |
When GitHub removes `work@acme-corp.com` from verified emails
Then the membership for `github/acme-corp` is disabled
And the membership for `github/client-org` remains active (its email is still present)
And only the affected membership is disabled
```

---

### Scenario 12: All GitHub emails removed — all org memberships disconnected

```gherkin
Given the user has only one verified email: `work@acme-corp.com`
And the user has memberships in tenants: [github/acme-corp, github/open-source-org]
When GitHub removes `work@acme-corp.com` from verified emails
Then all memberships derived from GitHub org association are disabled
And the personal tenant membership remains unaffected
And the user's console shows only the personal tenant (if active)
```

---

### Scenario 13: Email removal does not delete the tenant

```gherkin
Given a tenant `github/acme-corp` has multiple members
And the user's membership is disabled due to email removal
Then the tenant `github/acme-corp` still exists
And other members' memberships are unaffected
And the tenant remains available for other members
```

---

## Feature: Repo-to-tenant mapping

### Background

```gherkin
Given the user is authenticated via GitHub
And the user has active memberships in tenants
```

---

### Scenario 14: Repositories mapped to tenants on sign-in

```gherkin
Given the user belongs to GitHub org `acme-corp` with repositories:
  | repo         | visibility | access_level |
  | backend-api  | private    | admin        |
  | frontend-app | private    | write        |
  | docs         | public     | read         |
And a tenant `github/acme-corp` exists with an active membership
When the system processes repository mapping
Then the repositories are associated with tenant `github/acme-corp`
And the association uses the `org_slugs` field on the membership:
  | membership_tenant | org_slugs   |
  | github/acme-corp  | [acme-corp] |
And projects created from these repos are routed to the acme-corp tenant
```

---

### Scenario 15: Personal repos mapped to personal tenant

```gherkin
Given the user has personal repositories:
  | repo            | visibility |
  | my-side-project | private    |
  | my-blog         | public     |
And a personal tenant exists: `personal/<github_username>`
When the system processes repository mapping
Then personal repos are associated with the personal tenant
And the personal tenant's `org_slugs` includes the GitHub username
```

---

### Scenario 16: Org membership removed — repos unmapped

```gherkin
Given the user has repos mapped to tenant `github/acme-corp`
And the user's membership in `github/acme-corp` is disabled
When the system processes repository mapping
Then repos previously mapped to `github/acme-corp` are no longer routed there
And the repos may be reclassified as personal (if the user still has access)
And the `org_slugs` on the disabled membership is preserved for audit
```

---

## Feature: Tenant activation

### Background

```gherkin
Given the user has auto-created inactive tenants from GitHub orgs
```

---

### Scenario 17: User activates an auto-created tenant

```gherkin
Given the user has an inactive membership in `github/acme-corp`
When the user activates the membership via the console
Then the membership's `disabled_at` is cleared
And the membership becomes active
And the tenant is now visible in "My Dōjōs"
And projects can now be routed to this tenant
And the user's role is preserved from the auto-creation
```

---

### Scenario 18: User declines an auto-created tenant

```gherkin
Given the user has an inactive membership in `github/open-source-org`
When the user declines the membership via the console
Then the membership is soft-deleted or marked as declined
And the tenant is not visible in "My Dōjōs"
And no projects are routed to this tenant
And the tenant may be cleaned up if no other members exist
```

---

## Edge cases

### Scenario 19: GitHub org has no email domain match

```gherkin
Given the user belongs to GitHub org `anonymous-oss-org`
And the user has no email matching `anonymous-oss-org.com`
When the user signs in
Then a tenant `github/anonymous-oss-org` is created
And a membership is created with `kind = community`
And the tenant is inactive until activated
And no email-org association is created
```

---

### Scenario 20: Multiple users in the same GitHub org

```gherkin
Given users Alice and Bob both belong to GitHub org `acme-corp`
And both sign in to the Dōjō
When the system provisions tenants
Then a single tenant `github/acme-corp` is created (not duplicated)
And Alice has a membership in the tenant
And Bob has a separate membership in the same tenant
And both memberships are inactive until activated
And the tenant's member count reflects both memberships
```

---

### Scenario 21: GitHub OAuth scope changes

```gherkin
Given the user signed in with GitHub scope `read:user, user:email, read:org`
When the user re-authenticates with reduced scope (e.g., `read:user, user:email`)
Then org membership data may be incomplete
And the system preserves existing tenant/membership rows
And new orgs are only provisioned for the newly-visible orgs
And a warning is logged about incomplete org visibility
```

---

### Scenario 22: Race condition — concurrent sign-ins

```gherkin
Given the user opens two browser tabs and signs in simultaneously
When both sign-in flows execute concurrently
Then idempotent upserts ensure no duplicate tenants or memberships
And the `(tenant_id, user_id)` unique constraint on `dojo.memberships` prevents duplicates
And the `(provider, subject)` unique constraint on `dojo.identities` prevents duplicates
```

---

# Part II — Decisions, multi-forge identity, and the provisioning contract

> Part I above was written GitHub-first and assumes an `activation` flag gates
> everything. Both assumptions are revised here. Where Part I and Part II
> disagree, **Part II wins** — the scenarios in Part I that reference
> `disabled_at` as the gate (2, 3, 5, 17, 18, 19, 20) are superseded by §II.4
> and §II.5.

## II.0 Why this section exists

The implementation gap is total, not partial:

- **Nothing creates a tenant.** There are zero inserts into `dojo.tenants` in
  the whole app. `syncGithubMemberships` is built and tested but its contract is
  *"only tenants that already exist … never invents a tenant"* — it joins, it
  does not provision. The first user in an org has nothing to join.
- **Provisioning is not wired to sign-in.** The only caller is
  `POST /v1/you/github/sync`, an explicit endpoint.
- **That endpoint mostly no-ops.** It reads `session.provider_token`, which
  Supabase populates only immediately after the OAuth exchange. On any later
  call it returns `{ synced: false, reason: 'no_github_token' }` — silently.
- **The code contradicts Part I.** `addMember` sets neither `disabled_at` nor
  `sync_status`, so auto-provisioned memberships are created ACTIVE.

## II.1 Decisions

| # | Decision |
|---|---|
| D1 | The **personal dōjō is always active**. Every authenticated user has one, immediately, without any activation step. |
| D2 | **`disabled_at` is not the gate.** What gates governance and data sync is subscription + seat (§II.5). An unclaimed or unsubscribed org tenant may exist and be visible; it simply cannot sync private data. |
| D3 | An org tenant created by a **non-owner is UNCLAIMED**. It works at the free tier. Subscribing requires an owner to claim it (§II.4). |
| D4 | Repo→tenant mapping uses **both** sources: the forge API (authoritative for access level) and sensei's local discovery (authoritative for what the developer actually works on). §II.6. |
| D5 | **`origin = personal`** for personal tenants, not `org`. |
| D6 | The model is **forge-agnostic** from the start: GitHub, GitLab, Bitbucket, Azure DevOps. An organization may have **several** forge connections. §II.2. |

## II.2 The forge-agnostic model

Today `tenant_origin` is the enum `('github','org')` and `auth_method` carries
`github_oauth`. GitHub is baked into the type system, which is the trap D6 is
about.

**A tenant is an ORGANIZATION, not a forge org.** Forge identities attach to it:

```
dojo.tenant_connections
  id             uuid pk
  tenant_id      uuid not null references dojo.tenants(id) on delete cascade
  provider       forge_provider not null    -- github | gitlab | bitbucket | azure_devops
  external_id    text not null              -- the forge's STABLE org id, not the slug
  external_slug  text not null              -- display/matching only; can be renamed upstream
  connected_by   uuid not null              -- the user who linked it
  verified_at    timestamptz                -- when org control was last proven
  unique (provider, external_id)
```

`unique (provider, external_id)` is the anti-duplication rule: one forge org
maps to at most one tenant, forever. **Keyed on the forge's numeric/GUID id, not
the slug** — a slug is renameable and reusable upstream, so keying on it would
let a renamed-then-squatted org inherit another tenant's governance.

Enum changes:

- `tenant_origin` → `('personal', 'organization')`. What KIND of tenant, not
  which forge.
- new `forge_provider` → `('github', 'gitlab', 'bitbucket', 'azure_devops')`.
- `auth_method` gains `oauth` as the generic value; `github_oauth` is retained
  for the rows already written.

`tenants.key` stops being `github/{org}` and becomes a **user-visible slug,
globally unique, chosen at creation** (defaulting to the first connection's
slug). The forge no longer prefixes it, because a tenant can have several.

## II.3 Slug collision across forges

The problem: `sensei-hq` exists on GitHub AND on Azure DevOps. Same name. They
may or may not be the same organization — **nothing can prove it automatically.**
Same slug is not evidence: anyone can register `sensei-hq` on a forge nobody
else is using.

So linking is an **authorized human act**, and the proof is: *one person,
authenticated on both sides, who already administers the tenant.*

```gherkin
Scenario: Signing in from a second forge where the slug is already taken
  Given a tenant `sensei-hq` exists with a github connection
  When the user signs in via Azure DevOps and is an admin of azure org `sensei-hq`
  Then the system finds no tenant_connection for (azure_devops, <that org id>)
   And it finds an existing tenant whose slug is `sensei-hq`
   And the outcome depends on the caller's standing in that tenant:
     | caller standing in tenant `sensei-hq` | outcome                                            |
     | admin or owner                        | offered "link this Azure org to your sensei-hq dōjō" |
     | member, not admin                     | offered "ask an admin to link it"                   |
     | not a member                          | slug is TAKEN — offered a different slug            |
   And no automatic link is ever made on slug equality alone
```

For the non-member case the system proposes a free slug (`sensei-hq-2`,
`sensei-hq-azure`) and creates a **separate** tenant. Two orgs that genuinely
share a name stay separate, which is correct: they are separate organizations.

**Merging later.** Two tenants that turn out to be one org are merged by an
admin of both, moving connections/repos/memberships onto the surviving tenant.
Out of scope for the first implementation, but the model must not preclude it —
which is why connections are a child table rather than columns on `tenants`.

## II.4 Claim — replacing the activation flag

An org tenant records who, if anyone, has proven org ownership:

```
dojo.tenants
  + claimed_at    timestamptz
  + claimed_by    uuid
```

- **Unclaimed**: created by a non-owner. Exists, is visible, works at the free
  tier. **Cannot hold a subscription**, so it can never sync private data.
- **Claimed**: an owner/admin on any connected forge claimed it. They become
  tenant `admin` regardless of the role derived at auto-provision, and the
  tenant may subscribe.

```gherkin
Scenario: Non-owner creates the tenant, owner claims it later
  Given a plain member of github org `acme` signs in first
  Then tenant `acme` is created UNCLAIMED
   And that user's membership role is `contributor` (derived from the forge)
   And the tenant is on the free tier and cannot subscribe
  When a github OWNER of `acme` later signs in
  Then they are offered to claim the tenant
   And on claiming: claimed_by = that user, their membership role becomes `admin`
   And the tenant may now hold a billing account
```

The forge role is re-read at each sign-in, so a claim is verified against
current org standing, not a stale snapshot.

## II.5 — SUPERSEDED by §IV.3 and `daemon-sync.md` §8a (three denial vocabularies coexisted; §8a's is authoritative) What actually gates sync

Replaces `disabled_at` as the mechanism. One pure predicate, testable without a
database:

```
can_sync(tenant, repo, user) =
    tenant.origin = 'personal'                      → ALLOW   (always free)
  | repo.visibility = 'public'                      → ALLOW   (open source is free)
  | tenant unclaimed                                → DENY    (cannot subscribe)
  | billing.status <> 'active'                      → DENY    (org not subscribed)
  | no active seat for (tenant, user)               → DENY    (member not on a seat)
  | otherwise                                       → ALLOW
```

Notes that make this airtight:

- **Private repo in a public org still requires a subscription.** The gate is
  the REPO's visibility, not the org's — a private repo under an open-source org
  is private data.
- **Deny is honest, not silent.** A denied sync returns a reason
  (`unclaimed` | `not_subscribed` | `no_seat`) that the CLI and console surface.
  Silently syncing nothing is how the current `no_github_token` no-op became
  invisible for two days.
- **The gate is evaluated dōjō-side**, on every write. A daemon that believes it
  has a seat is not evidence; the service decides.
- `dojo.seats` already carries `(tenant_id, user_id, namespace_id, ended_at)`
  with a unique active-seat index, and `dojo.billing_accounts` already carries
  `status`/`seats_included`/`seats_used`. No new billing modelling is required.

## II.6 Repo → tenant mapping, from both sides

**Forge API (authoritative for access level).** Listing `/user/repos` and the
org's repos yields `visibility` and the caller's `permissions`. Used to decide
which tenant a repo belongs to and what the user may do with it.

**Local discovery (authoritative for what is actually worked on).** sensei
discovers repositories on disk first and syncs their identity on connect — the
established rule. Mapping a local repo to a tenant is by **remote URL**:

```
git remote → normalise → (provider, external_org_slug, repo_slug)
          → tenant_connections lookup on (provider, external_org_id/slug)
          → tenant
```

The normaliser must handle the forms each forge emits, including SSH/HTTPS and
Azure's two shapes:

| forge | remote | org |
|---|---|---|
| github | `git@github.com:acme/api.git` | `acme` |
| gitlab | `https://gitlab.com/acme/sub/api.git` | `acme` (top-level group) |
| bitbucket | `git@bitbucket.org:acme/api.git` | `acme` |
| azure_devops | `https://dev.azure.com/acme/proj/_git/api` | `acme` |
| azure_devops | `acme@vs-ssh.visualstudio.com:v3/acme/proj/api` | `acme` |

A repo whose remote matches no connection is **unmapped, not personal** — it
stays local-only until its org is connected. Defaulting it to the personal
tenant would silently move an employer's private repo into a free personal dōjō.

## II.7 The provisioning contract

One idempotent operation, three callers — so "in sync regardless of where it was
initiated" is a property of the design rather than two flows kept in step.

```
ensureProvisioned(userId, forgeToken?, provider) -> ProvisionResult
  1. upsert dojo.identities  (provider, subject = forge user id)
  2. ensure personal tenant  (origin=personal, key=<login>, ACTIVE, claimed_by=user)
  3. for each forge org the token proves:
       find tenant via tenant_connections (provider, external_id)
       └ none → create tenant (unclaimed unless caller is owner) + connection
       ensure membership, role derived from forge org role
  4. map repositories (§II.6)
  5. return { personal, tenants[], memberships[], repos[], denied[] }
```

Callers:

| caller | when |
|---|---|
| web sign-in callback | every sign-in |
| `POST /v1/auth/cli/token` | when the daemon completes device auth |
| `POST /v1/you/github/sync` | explicit re-sync from the console |

**Idempotency** is by `(provider, external_id)` on connections,
`(tenant_id, user_id)` on memberships and `(provider, subject)` on identities —
all already unique — so concurrent sign-ins converge (Part I Scenario 22).

**Token availability.** `provider_token` exists only immediately after the OAuth
exchange. Therefore provisioning **must** run in the sign-in callback, where the
token is in hand. Later calls without a token degrade to "refresh what we can
from the DB" and MUST report `synced: false` with a reason rather than appearing
to succeed.

## II.8 sensei ↔ dōjō sync

`ProvisionResult` is what sensei mirrors on connect. The daemon holds no
authority: it caches tenants/memberships/seat state for routing, and the dōjō
re-decides on every write.

```gherkin
Scenario: Initiated from sensei
  When the daemon completes device auth
  Then the dōjō runs ensureProvisioned and returns ProvisionResult
   And the daemon mirrors it into sensei.dojo_memberships
   And the daemon pushes its locally-discovered repositories
   And the dōjō maps each to a tenant (§II.6) and returns the mapping + any denials

Scenario: Initiated on the web
  Given the user signed in on the web and tenants were provisioned there
  When the daemon later connects
  Then ensureProvisioned is a no-op for what already exists
   And the daemon receives the same ProvisionResult shape
   And local state converges to it without a separate reconciliation path
```

## II.9 Migration of what already exists

- `tenant_origin`: add `personal`/`organization`; backfill `github`→
  `organization`, `org`→`organization`; then retire the old labels.
- Existing `github/{org}` tenants: keep `key` as-is (it is unique and in use),
  create the matching `tenant_connections` row from the `org` column, and
  populate `external_id` by a one-time lookup. A tenant whose external id cannot
  be resolved stays connected by slug and is flagged, not guessed.
- Existing memberships are unaffected; `claimed_at` starts NULL, so every
  pre-existing org tenant is unclaimed until an owner claims it. That is the
  correct default — none of them ever proved ownership.

---

# Part III — Adversarial review of Part II

Attacked against the live DDL rather than read for agreement. Three findings are
**blocking**: the spec as written cannot be implemented correctly.

## F1 — BLOCKING. The personal slug collides with the org slug

§II.2 dropped the `personal/` and `github/` prefixes and made `tenants.key` "a
user-visible slug, globally unique". `dojo.tenants` has exactly one unique
constraint on `key`. So a user whose login is `acme` gets personal tenant
`acme`, and the org `acme` can then never be provisioned — or worse, the order
decides who wins.

Part I avoided this with prefixes; Part II reintroduced it while removing them
for a different reason (a tenant has many forges, so a forge prefix is wrong).

**Both goals are satisfiable.** The first fix proposed a reserved sigil
(`@jerrythomas`); §IV.7 supersedes it with a better one — keep the prefix but
make it the origin KIND rather than the forge, so the key is
`personal/jerrythomas` vs `organization/sensei-hq`. See §IV.7 for why that is
strictly better.

## F2 — BLOCKING. The seat gate is keyed on the wrong grain

§II.5 denies when there is "no active seat for `(tenant, user)`". `dojo.seats`
is keyed `(user_id, namespace_id)` — **per project, not per tenant** — with
`namespace_id NOT NULL REFERENCES sensei.namespaces`.

Two consequences the predicate cannot express as written:

1. The gate must be evaluated per project, not per tenant.
2. **A seat cannot exist before the project does**, and the project is created
   by the very sync the seat is gating. The ordering is circular.

Resolve by deciding what the first sync of a new private project does: create
the namespace and an unbilled seat pending confirmation, or deny until the
project is created through the console. It cannot be left implicit.

## F3 — BLOCKING. Nothing creates a seat

The gate denies until an active seat exists. Neither Part II nor the code
defines how one comes into existence — `billing-data.ts` reads, counts and ends
seats. Without a creation path the private-sync gate is **closed permanently**,
which will present exactly like today's bug: a silent no-op that looks like
"nothing to sync".

Needs an explicit answer: auto-seat on first activity up to `seats_included`
(and what happens at the cap), or admin-assigns-seats, or self-serve claim.

## F4 — HIGH. Two visibility sources, no precedence

§II.5 keys the gate on `repo.visibility`. `dojo.seats.namespace_id` documents
that "its visibility (private/public) decides whether this participation is
billable". A private repo inside a public project — or the reverse — has two
answers.

State which is authoritative. Suggest: **the repo**, because it is the thing
whose contents are being synced, with the namespace deciding only *billability*.
But it must be written down; today it is two rules that happen not to have
disagreed yet.

## F5 — HIGH. No de-provisioning when someone leaves an org

Part I covers *email* removal (Scenarios 10–13). Nothing covers the forge case:
the user is removed from the GitHub org. `syncGithubMemberships` is explicit
that it "never removes", and §II.7 does not either.

So an ex-employee retains their membership, their tenant visibility and
potentially their seat indefinitely. That is a security defect, not untidiness.
Needs: on each provisioning pass, memberships whose forge org is no longer
proven get disabled and their seat ended — with the tenant untouched (Part I
Scenario 13 already has the right instinct).

## F6 — MEDIUM. A claim can outlive the claimer's standing

§II.4 says the forge role is re-read each sign-in, but nothing revokes a claim
when the claimer loses ownership or leaves the org. A tenant can end up claimed
— and therefore subscribable — by someone with no current standing.

## F7 — MEDIUM. The migration contradicts its own constraint

§II.9 says a tenant whose `external_id` cannot be resolved "stays connected by
slug and is flagged". §II.2 declares `external_id text not null` inside
`unique (provider, external_id)`. An unresolvable row cannot be written at all.

Either `external_id` is nullable with a partial unique index, or unresolved
tenants get no connection row and are listed for manual repair. The second is
safer: a connection is a claim of identity, and a guessed one is worse than
none.

## F8 — MEDIUM. The CLI path may have no forge token

§II.7 lists `POST /v1/auth/cli/token` as a provisioning caller, and separately
states provisioning "must run in the sign-in callback, where the token is in
hand". The daemon's device flow authenticates against the **dōjō**, so whether a
GitHub `provider_token` is present at that point is unverified.

If it is not, a CLI-initiated first connection can only mirror what the web
already provisioned — which directly contradicts the goal that initiation from
sensei creates the org. **This needs to be tested before implementation
proceeds**; it determines whether the daemon must drive the user through a web
sign-in on first connect.

## Depth assessment

Part II is deep enough on the **model** (forge-agnostic tenants, connections
keyed on stable external ids, claim replacing activation, both-sides repo
mapping) and on the **collision** question, which was the hard one.

It is **not yet deep enough to implement**, because F1–F3 mean the central
promise — private org data syncs when subscribed and seated — has no working
mechanism. F5 additionally means the first implementation would ship a
known-open access hole.

Recommended order once F1–F3 are settled: the personal-dōjō path first (it has
no billing, no claim and no seat, so it closes the hole for every new user and
is fully testable end to end), then claim, then the seat/billing gate, then
de-provisioning, and only then the second forge.

---

# Part IV — Seats, subscription and the resolution of F1–F5

## IV.0 The insight that unblocks F2/F3

`dojo.seats` is doing **two jobs at once**, which is why the gate looked
circular:

| concept | question | grain | who creates it |
|---|---|---|---|
| **participation** | who is actually working where | `(user, namespace)` | observed, by activity |
| **entitlement** | who is licensed to sync private data | `(tenant, user)` | granted, by an admin |

The existing table is *participation* — its own comment says the namespace's
visibility "decides whether this participation is billable", and it carries
`last_active_at`. It is a measurement.

Entitlement is a **new** table. Splitting them dissolves the circularity in F2:
participation rows appear from ordinary work (public repos and projects sync
without any subscription), and the private gate is a separate entitlement check
that never has to exist before the project does.

## IV.1 The flow, end to end

```gherkin
Scenario: Private org from creation to synced private repos
  Given an owner creates a private org dōjō and claims it
   And members join and repositories are connected
  When there is no active subscription
  Then public repositories sync — metrics and governance included
   And private repositories are shown DISABLED and excluded from sync
   And projects may still be created and repositories connected to them
   And the console surfaces: "X users are working in your private repositories;
       you have Y members. Syncing them needs at least X seats."
  When an admin sets up billing and buys N seats
  Then N seat allocations exist for the tenant, initially unassigned
  When the admin allocates seats to specific users
  Then those users' private repositories unlock and sync
   And members without an allocation still sync only public repositories
```

Seat assignment is **explicit**. Nothing auto-grants, so the bill can never grow
because someone opened a repo.

## IV.2 `dojo.seat_allocations` — entitlement, with history

```
dojo.seat_allocations
  id                  uuid pk
  tenant_id           uuid not null references dojo.tenants(id) on delete cascade
  billing_account_id  uuid not null references dojo.billing_accounts(id)
  user_id             uuid                      -- NULL = purchased, not yet assigned
  allocated_at        timestamptz not null default now()
  allocated_by        uuid                      -- the admin who assigned it
  released_at         timestamptz               -- NULL = current; set = historical
  release_reason      seat_release_reason       -- transferred | revoked | member_left
                                                -- | subscription_ended | seats_reduced
  create unique index seat_alloc_current_user_idx
      on dojo.seat_allocations (tenant_id, user_id)
   where released_at is null and user_id is not null;
```

**Current + past by row, never by mutation.** A transfer releases the old row
(`released_at`, `release_reason='transferred'`) and inserts a new one. The
history answers "who held this seat in March", which a mutated column cannot.

Invariant: `count(*) where released_at is null` ≤
`billing_accounts.seats_included`. Enforced on allocate, and on any reduction of
`seats_included` (§IV.5).

## IV.3 The gate, corrected

Supersedes §II.5. F4 is resolved as suggested: **the repo governs sync, the
namespace governs billability.**

> **CORRECTED 2026-08-28 — this was only half the gate.** Everything below
> answers *may this sync?* Nothing here answers *did anyone choose to?*, and the
> two are independent. See `docs/requirements/repository-sharing.md`.

**Two questions, both required.** A repository syncs only when entitlement AND
election both say yes.

```
                 ENTITLEMENT — may it?  (the dōjō decides)
can_sync(repo, user, tenant) =
    repo.visibility IS NULL                           → DENY  forge_visibility_unknown
  | repo.visibility = 'public'                        → ALLOW   (open source is free)
  -- NO `origin = 'personal' → ALLOW`. Entitlement keys on VISIBILITY, not on who
  -- owns the tenant: a PRIVATE repository is subscription-gated whoever owns it,
  -- including a solo developer's own. The earlier unconditional personal-ALLOW
  -- contradicted §2a ("private stays local-only regardless of configuration") and
  -- would have hosted every personal private repo free — the common case, since no
  -- personal tenant carries a billing row.
  -- FAIL CLOSED ON ABSENCE. Each of the next three is a MISSING-ROW test that
  -- must precede its value test, because `NULL <> 'active'` is NULL, not TRUE —
  -- so a value test alone falls through to ALLOW. Verified: 3 live tenants, 0
  -- billing_accounts rows, and the composite ALLOWED an org-mandated private
  -- repo on no subscription at all.
  | no billing_accounts row for tenant                → DENY  not_subscribed
  | period_start IS NULL OR period_end IS NULL        → DENY  not_subscribed
  | tenant.claimed_at IS NULL                         → DENY  unclaimed
  | billing.status <> 'active'                        → DENY  not_subscribed
  | now() NOT BETWEEN period_start AND period_end     → DENY  subscription_expired
  | no CURRENT seat_allocation for (tenant, user)     → DENY  no_seat
  | otherwise                                         → ALLOW

                 ELECTION — did anyone choose it?  (authority decides)
authority(repo, tenant) =
    tenant.origin = 'organization' AND repo.visibility <> 'public'  → ORG
  | otherwise                                                      → USER

elected(repo, user) =
    authority = ORG   → the org's policy for this repo   (MANDATORY: the user
                        cannot switch it off, locally or in the console)
  | authority = USER  → this user's election for this repo

                 THE GATE
may_share = can_sync(...) AND elected(...)
```

**Two functions, two different inputs — and origin appears in only one:**

```
entitlement = f(forge visibility, subscription)   -- may it? — origin is irrelevant
authority   = f(origin, forge visibility)         -- who decides? — origin decides this
```

That separation is the simplification decision 2 bought: `public` is free for
anyone, `private` is paid for by everyone, and *who owns the tenant* only ever
answers the second question.

**Authority follows who owns the code and who pays.** A personal repo of either
visibility, and an org's PUBLIC repos, are the user's to elect — the org is not
paying for open source, and a contributor's own metrics are their own. An org's
PRIVATE repos are the org's, on the org's subscription.

**A mandate is an election, not an entitlement.** It cannot conjure one: an
org-mandated repo on a lapsed subscription still DENIES `not_subscribed`. This is
the trap in collapsing the two — "the org said share" is not "the org may".

**A mandate overrides the daemon's local gate 1.** §V.3's "the daemon never even
asks about a repo the user did not opt in" holds for every repository where the
USER has authority, and not for org-mandated ones. That narrows a previously
absolute promise, deliberately: *nothing leaves the machine without local consent*
becomes *…without local consent, or an organization's mandate on that
organization's own private code*. Recorded rather than discovered.

Every DENY carries its reason to the CLI and console, **and says which of the two
questions refused** — entitlement or election. A denial that reads as "nothing to
sync" is the failure mode that hid the `no_github_token` no-op for two days; and
an "off" that does not say whether the user or the org turned it off is the same
failure one level up.

Note the gate never consults `dojo.seats`. Participation informs the
*recommendation*, never the decision.

## IV.4 The seat recommendation

```
seats_needed(tenant) =
  count(DISTINCT s.user_id)
    FROM dojo.seats s
    JOIN sensei.namespaces n ON n.id = s.namespace_id
   WHERE s.tenant_id = $tenant
     AND s.ended_at IS NULL
     AND n.visibility = 'private'
     AND s.last_active_at > now() - <activity window>
```

This is what produces "X users are working in your private repositories". It
counts **observed private participation**, which is exactly the population that
would need a seat — not total membership, which would over-quote every org with
dormant members.

## IV.5 Lifecycle — the cases that break naive designs

| event | effect |
|---|---|
| **Subscription lapses** (`status` → `past_due`/`canceled`) | Allocations are **retained**, not released. The gate denies on status, so private sync stops immediately; re-subscribing restores the assignment work rather than making the admin redo it. |
| **Period rolls over** | Allocations carry across. `billing_account_id` pins which subscription funded a seat, so a plan change is auditable. |
| **Seats reduced below current allocations** | Cannot silently over-allocate. The reduction is **refused** unless the admin releases the excess first (`release_reason='seats_reduced'`). Auto-picking whom to cut is not the service's decision. |
| **Member leaves the forge org** | Membership disabled (F5), allocation released with `member_left`, seat returns to the pool. |
| **Member removed from the tenant** | Same, `revoked`. |
| **User transferred** | Old row released `transferred`, new row inserted. Seat count unchanged. |
| **Tenant unclaimed** | Cannot hold a billing account at all, so no allocations can exist. |

## IV.6 F5 — de-provisioning, now concrete

Each provisioning pass compares proven forge orgs against existing memberships:

```gherkin
Scenario: A member leaves the forge org
  Given the user has an active membership in tenant `acme` via github
   And a current seat allocation
  When a provisioning pass runs and GitHub no longer proves that org
  Then the membership is disabled (disabled_at set)
   And the seat allocation is released with reason `member_left`
   And the tenant and other members are untouched
   And the freed seat is immediately re-allocatable
```

**Only a pass that positively proved the forge list may de-provision.** A failed
or token-less call must never be read as "the user left everything" — that
would disable an entire org on a GitHub outage. This is the fail-closed rule
pointing the other way, and it is the single most dangerous part of the flow to
get wrong.

## IV.7 F1 — resolved, and without a sigil

An earlier draft proposed `@jerrythomas` for personal tenants. **Not needed**,
and it would have been the wrong shape.

`dojo.tenants.key` already carries the origin as a prefix — its own comment says
the canonical form is `"<origin>/<org>[/<dojo>]"` — and `dojo-auth.ts` resolves a
tenant by joining the two URL segments: `.eq('key', ${origin}/${org})`. There are
**33 routes** under `/v1/t/[origin]/[org]/`.

So the collision in F1 was caused by Part II removing the prefix, not by the
prefix existing. Part II removed it for a good reason — a *forge* prefix is
wrong once a tenant has several forges — but the fix is to change what the
prefix means, not to delete it:

| tenant | key | URL | origin |
|---|---|---|---|
| personal | `personal/jerrythomas` | `/t/personal/jerrythomas` | `personal` |
| organization | `organization/sensei-hq` | `/t/organization/sensei-hq` | `organization` |

This satisfies every constraint at once:

- **No collision.** `personal/x` and `organization/x` differ under the existing
  `unique (key)`. No constraint change.
- **No forge in the key**, so Part II's objection is met — the key is stable
  across however many forges connect, which is *better* than today where the
  forge is baked into the URL.
- **No `@`**, so the URL question does not arise. (`@` is legal in a path
  segment per RFC 3986 §3.3, but it is the userinfo delimiter in an authority
  and gets linkified as a mention by some clients — avoidable weirdness for no
  gain.)
- **All 33 routes keep their shape.** Only the values `origin` takes change.

**Migration cost, stated plainly.** Existing keys `github/{org}` must be
rewritten to `organization/{org}`, because resolution joins the segments and
`origin` no longer says `github`. That changes those tenants' URLs. Given how
few exist today this is the moment to do it; after launch it would need a
redirect table.

## IV.8 Still open

- **F8 — RESOLVED, and it inverts the concern.** The daemon does receive
  `provider_token` from the exchange and **persists it to the OS keychain**
  (`dojo_client/session.rs::store_provider_token`, keyed
  `provider_token.<persona>`), already reports `canReadOrgs` in
  `/api/auth/status`, and already uses it (`github_verified_emails`). A
  `provider_refresh_token` slot exists too.

  So the CLI path has **better** token durability than the web path: the web's
  `session.provider_token` evaporates after the exchange, while the daemon's is
  durable. A CLI-initiated FIRST connection can provision orgs, and the daemon
  is arguably the more reliable driver of the org sync.

  **Design consequence.** The daemon holds the token; the dōjō makes the
  decisions. The daemon must therefore send the `provider_token` to the dōjō on
  connect (over TLS, alongside the access token it already sends) and let the
  dōjō verify the org list itself. The daemon must NOT read the orgs and send a
  list — that would make the service trust a client's claim about its own
  entitlements, which is the one thing §II.5 says it must never do.

  Remaining empirical check: that Supabase actually returns `provider_token` on
  the PKCE exchange for this provider. The code handles it as `Option` and
  reports `canReadOrgs: false` when absent, so a miss is visible rather than
  silent — but it has not been observed on a live sign-in. The daemon is
  currently signed out.
- **Proration** — a seat allocated mid-cycle. A billing question, not a gate
  question; the gate only asks "is there a current allocation".
- **Downgrade UX** — refusing a seat reduction is correct but needs a console
  flow that shows which allocations to release.

---

# Part V — Database design and the per-repo sync decision

Design covers all three phases; the phase column says when each piece is needed.

## V.0 A vocabulary collision the gate depends on

The gate keys on repository visibility, and the two sides do not agree:

| | values | meaning |
|---|---|---|
| `sensei.repo_visibility` (enum) | `private` \| `shared` | `shared` = shared WITH the dōjō |
| `dojo.repositories.visibility` (text+CHECK) | `private` \| `public` | `public` = publicly visible ON THE FORGE |

These are different questions. A private GitHub repo that the user has chosen to
share with their dōjō is `shared` locally and `private` upstream — and the gate
must read the *forge* answer, or every shared private repo would sync free.

**Resolution.** Introduce `sensei.forge_visibility` (`public` | `private` |
`internal`) as the forge's answer, distinct from the local sharing intent.
`dojo.repositories.visibility` becomes that enum too (house rule: enums, not
text+CHECK). `sensei.repositories.visibility` keeps its current meaning and is
renamed in comment, not in type, to avoid a breaking migration.

`internal` matters for Phase 3: GitHub/GitLab "internal" repos are visible to an
enterprise but are not public, so they gate as private.

## V.1 dōjō — new types

| object | definition | phase |
|---|---|---|
| `dojo.forge_provider` | enum `github \| gitlab \| bitbucket \| azure_devops` | 2 |
| `dojo.forge_visibility` | enum `public \| private \| internal` | 2 |
| `dojo.seat_release_reason` | enum `transferred \| revoked \| member_left \| subscription_ended \| seats_reduced` | 3 |
| `dojo.tenant_origin` | **add** `personal`, `organization`; retire `github`, `org` after backfill | 1 |
| `dojo.auth_method` | **add** `oauth` (generic); keep `github_oauth` for written rows | 2 |

## V.2 dōjō — table changes

**`dojo.tenants`** — phase 1 for the sigil, 3 for the claim:

```
+ claimed_at   timestamptz          -- NULL = unclaimed; cannot hold billing
+ claimed_by   uuid                 -- who proved forge ownership
```

`key` convention: `personal/{login}` and `organization/{slug}` (§IV.7). The
existing single unique on `key` is sufficient and unchanged — the origin prefix
keeps the two namespaces apart, and all 33 `/v1/t/[origin]/[org]/` routes keep
their shape.

**`dojo.tenant_connections`** — NEW, phase 2:

```
id            uuid pk
tenant_id     uuid not null references dojo.tenants(id) on delete cascade
provider      dojo.forge_provider not null
external_id   text not null        -- the forge's STABLE id, never the slug
external_slug text not null        -- display + matching only
connected_by  uuid not null
verified_at   timestamptz          -- when org control was last proven
created_at    timestamptz not null default now()
unique (provider, external_id)
```

One forge org maps to at most one tenant, forever. Keyed on the stable id
because a slug can be renamed and re-registered upstream; keying on the slug
would let a squatter inherit another tenant's governance.

**`dojo.seat_allocations`** — NEW, phase 3. Shape in §IV.2.

**`dojo.repositories`** — phase 2:

```
~ visibility   text+CHECK  →  dojo.forge_visibility
+ provider     dojo.forge_provider          -- which forge this repo lives on
+ external_id  text                         -- stable forge repo id
```

## V.3 Three gates, not two

`sensei.repo_visibility` is an enum `('private','shared')` and its own comment
says what it is: *"Whether a repository participates in sync at all … a repo the
user never wanted shared should not start syncing merely because they signed
in."* That is **user intent**, and it is a separate question from both forge
visibility and entitlement.

The full chain, in order:

| # | gate | question | owned by | evaluated |
|---|---|---|---|---|
| 1 | **intent** | did the user opt this repo in? | `sensei.repositories.visibility = 'shared'` | daemon, locally |
| 2 | **cost** | is it public, private or internal on the forge? | `dojo.repositories.visibility` (text+CHECK; promoted to a `dojo.forge_visibility` enum in phase 2) | dōjō |
| 3 | **entitlement** | claimed, subscribed, seated? | claim + billing + `seat_allocations` | dōjō |

> **NARROWED — see `docs/spec/dojo/daemon-sync.md` §8a.** Gate 1 is sovereign for
> repositories where the USER holds authority, and NOT for org-mandated ones.
> `seat_allocations` in the table above is also unbuilt (phase 3).

Gate 1 is local and comes first: the daemon never even asks about a repo the
user has not shared. Gates 2 and 3 are the dōjō's and are never mirrored.

## V.4 The daemon asks; it does not remember

An earlier draft cached the dōjō's per-repo ruling on `sensei.repositories`
(`sync_allowed`, `sync_reason`, `sync_decided_at`). **Rejected.** A cached
entitlement is a second source of truth for something the service must own, and
it forced a TTL whose only job was to bound how wrong the cache could be.

Instead the daemon asks for a **sync plan** before each cycle:

```
GET /v1/t/{tenant}/sync/plan
  → { "allowed": [ { "repo_key": "github.com/acme/api", "repo_id": "…" } ],
      "denied":  [ { "repo_key": "github.com/acme/secret", "reason": "no_seat" } ] }
```

The daemon then pushes metrics and pulls governance for `allowed` only.

Why this is better than the cache:

- **No staleness, so no TTL.** A revoked seat bites on the next cycle rather
  than after a timeout chosen to be a compromise.
- **Allow-list, not per-repo permission check.** The daemon syncs the set it was
  handed. It cannot accidentally include a repo it never asked about, which a
  "may I sync X?" shape permits by omission.
- **Nothing to keep in step.** No column can disagree with the dōjō, because no
  column holds the answer.
- **Offline degrades correctly.** No plan, no sync. Fail-closed falls out of the
  design rather than needing a nullable boolean whose NULL means "no".
- **`denied[]` carries the reason**, so the CLI and console can say *why* a repo
  is dark — `no_seat`, `not_subscribed` and `unmapped` are three different
  problems — without the daemon storing a decision it does not own.

**Scope.** The plan governs **metrics and governance only**, not the repository
list itself. Repo identity is registered separately on connect (the established
local-first rule); the plan is the entitlement filter applied on top.

**Still enforced at the write.** The plan is an optimisation that stops the
daemon shipping data that will be refused; the dōjō re-evaluates on every write
and the refusal carries its reason. A daemon that ignores the plan gains
nothing.

### sensei schema delta — almost none

Because the decision is not stored, `sensei.repositories` needs only what it
already has (`repo_key`, `remote_url`, `dojo_id`, `visibility`) plus:

```
+ forge_visibility  sensei.forge_visibility   -- display only; the dōjō still decides
```

`forge_visibility` is for the console to grey out a private repo before a
round-trip. It is **not** consulted by any sync decision — if it drifts, nothing
is mis-synced, only mis-drawn.

## V.5 What each phase needs

| phase | dōjō | sensei |
|---|---|---|
| **1 · personal** | `tenant_origin` += personal/organization; `personal/{login}` key convention; provisioning writes tenant + membership; `GET /sync/plan` returning everything shared | consume the plan; sync only `allowed`. No schema change at all. |
| **2 · public org** | `forge_provider`, `forge_visibility`, `tenant_connections`, repo `provider`/`external_id`; repo→tenant mapping; plan denies private with `not_subscribed` | `forge_visibility` (display only); mapping by remote URL (§II.6) |
| **3 · private org** | `claimed_at`/`claimed_by`, `seat_allocations`, `seat_release_reason`, the full gate + de-provisioning; plan denies with `no_seat` / `subscription_expired` | surface `denied[].reason` in the CLI |

The sensei side barely changes across all three phases, which is the strongest
argument for the plan endpoint: the entitlement model can grow from "everything"
to claim-plus-billing-plus-seats without the daemon learning anything new. It
asks the same question every cycle and does as it is told.

Phase 1 is shippable alone and closes the hole for every new user: no billing,
no claim, no seats, no second forge. Phases 2 and 3 add tables rather than
reshaping phase 1's, which is the point of designing all three now.

---

# Part VI — F6 and F7 resolved; two phases

## VI.1 F6 — claim is decoupled from admin

The error was treating a *proof* as a *property*. Forge ownership is a fact that
changes; a claim records that it was true once.

**Claim and admin become separate things.**

| | claim | admin |
|---|---|---|
| means | this tenant was legitimately provisioned, and someone proved org control | this person may administer the tenant |
| holders | one, recorded on `dojo.tenants` | many, via `dojo.memberships.role` |
| gates | whether the tenant may hold a billing account | seats, governance, members |

```
dojo.tenants
  + claimed_at    timestamptz
  + claimed_by    uuid
  + claim_state   dojo.claim_state   -- unclaimed | claimed | stale
```

Rules:

- **Claiming grants admin; losing the claim does not remove it.** The claimer
  becomes a tenant admin. If their forge standing later lapses the claim goes
  `stale`, but their admin role is untouched at that moment — revoking mid-cycle
  is how an org gets locked out of its own dōjō.
- **Re-verified on every provisioning pass.** If the claimer no longer proves
  owner/admin on any connected forge, `claim_state = 'stale'` and the tenant's
  admins are notified. Only a pass that positively proved the forge list may do
  this (§IV.6) — an outage must not stale every claim.
- **A stale claim is takeover-able.** Any user who currently proves owner/admin
  on a connected forge may claim it, becoming `claimed_by` and an admin. This is
  the path for acquisition, for a departed founder, and for "she left six months
  ago".
- **A stale claim keeps an existing subscription running** but blocks new
  billing changes. Cutting off a paying org because one person changed jobs is
  the wrong failure.
- **The last admin cannot be removed.** Same principle as refusing a seat
  reduction (§IV.5): the service must not let you create an unadministrable
  tenant.

```gherkin
Scenario: The claimer leaves the company
  Given tenant `organization/acme` is claimed by Alice, a github owner
   And Bob is also a tenant admin
  When a provisioning pass proves Alice is no longer an owner of the github org
  Then claim_state becomes `stale`
   And Alice's admin role is NOT automatically removed
   And the tenant's admins are notified that the claim needs re-proving
   And the existing subscription continues
   And any current github owner may take over the claim

Scenario: The claimer was the only admin
  Given tenant `organization/acme` is claimed by Alice, the only admin
  When Alice's forge standing lapses
  Then claim_state becomes `stale`
   And Alice retains admin, so the tenant is never unadministrable
   And a current forge owner may take over the claim and become admin
```

## VI.2 F7 — org identity moves down to the connection

`dojo.tenants.org` is documented as *"The GitHub org id (e.g. sensei-hq) or the
custom org name"* — a GitHub-era column. With several forges per tenant, the
forge's name for an org belongs on the **connection**, not the tenant.

| column | new meaning |
|---|---|
| `dojo.tenants.org` | the **tenant's own slug** — the URL segment, tenant-owned, unrelated to any forge |
| `dojo.tenant_connections.external_slug` | **the forge's name** for that org. NOT NULL — it is how the org was found |
| `dojo.tenant_connections.external_id` | the forge's **stable id**. **NULLABLE** — an enrichment that arrives when the API confirms it |

This is what lets one tenant `organization/sensei-hq` connect to GitHub
`sensei-hq` and Azure `senseihq` — different upstream names, one tenant slug.

```
unique (provider, external_id) where external_id is not null
unique (provider, lower(external_slug)) where external_id is null
```

The first keeps the "one proven forge org → one tenant, forever" guarantee. The
second stops two *unproven* connections racing for the same slug.

**The squatting risk is contained by the gate, not by NOT NULL.** An
unverified connection (`verified_at IS NULL`) confers **no entitlement**: a
tenant whose only connection is unverified cannot be claimed, therefore cannot
hold billing, therefore cannot sync private data. A renamed-and-squatted org can
at worst occupy a slug — it can never inherit governance or reach private code.

That is strictly better than the earlier NOT NULL, which made the migration
impossible to write (§F7) and would have forced a guessed id or a dropped row.

## VI.3 Two phases

Auto-provisioning is cheap; entitlement is load-bearing. So the three-phase split
collapses:

**Phase 1 — provisioning.** Everything is provisioned and everything syncs;
nothing is gated yet.

- `tenant_origin` → `personal | organization`; key migration `github/{org}` →
  `organization/{org}`
- `tenants.org` re-documented as the tenant slug
- `forge_provider`, `tenant_connections` (with the nullable `external_id`)
- `ensureProvisioned` wired into all three callers (§II.7)
- repo→tenant mapping by remote URL (§II.6)
- `GET /v1/t/{tenant}/sync/plan` returning every shared repo as `allowed`

**Phase 2 — entitlement.** The gate arrives; the daemon does not change.

- `claim_state`, `claimed_at`, `claimed_by`; claim/takeover flows
- `forge_visibility`; `seat_allocations`, `seat_release_reason`
- billing wiring, the full `can_sync` (§IV.3), the seat recommendation (§IV.4)
- de-provisioning (§IV.6) and the F6 lifecycle

The daemon is unchanged between the phases — it asks for a plan and syncs what
it is given. All the new logic lands behind that one endpoint, which is what the
plan design bought.

---

# Part VII — `tenants.org` → `tenants.slug`

Call it what it is. The column is the tenant's own name in a URL; it has not
been "the org" since forge identity moved down to `tenant_connections` (§VI.2).

```
dojo.tenants
  ~ org  text not null   →   slug  text not null
```

New comment: *"The tenant's own slug — the second segment of its discovery path
`{origin}/{slug}`. Tenant-owned and forge-independent: the forge's name for an
org lives on `dojo.tenant_connections.external_slug`, and one tenant may connect
to forges that spell it differently."*

## VII.1 What changes

| surface | change | note |
|---|---|---|
| `database/ddl/table/dojo/tenants.ddl` | column + comment | the rename itself |
| `database/ddl/table/staging/tenants.ddl` | `org` → `slug` | staging mirrors the target |
| `database/ddl/procedure/staging/import_tenants.ddl` | 3 references | insert list, select list, `on conflict` update |
| `database/import/**/tenants.jsonl` | `"org"` → `"slug"` | seed data; currently one row (`global-dojo`) |
| `dojo/src/routes/v1/t/[origin]/[org]/` | dir → `[slug]` | **33 routes** |
| `dojo/src/lib/server/dojo-auth.ts` | param name | see VII.2 |

## VII.2 The route param is a different thing with the same name

`dojo-auth.ts` resolves a tenant with `.eq('key', ${origin}/${org})` — it never
SELECTs the column. The `org` there is the **route parameter**, which happens to
share a name with the column.

So the column rename does not require the route rename. They are separated here
deliberately:

- **The column rename is required** — the name is now wrong.
- **The route rename is cosmetic** but should ride along, because leaving
  `[org]` in the path while the column says `slug` reintroduces exactly the
  confusion this is fixing. It is 33 directory renames and one param, with no
  behaviour change and no URL change (the *value* in that position is
  unchanged).

## VII.3 What is NOT renamed

`dojo.memberships.org_slugs` **keeps its name.** It genuinely holds forge org
slugs — the set a membership covers, used for repo routing — so `org` is correct
there. Renaming it would be the opposite error.

## VII.4 Migration — there isn't one

The dōjō is **pre-release**, so the schema is reset rather than migrated. That
removes the whole problem class:

- `tenant_origin` is declared as its target — `('personal', 'organization')` —
  with no transitional labels to retire later.
- No `apply/after` data migration for keys or origins. The seed carries the
  final values.
- No out-of-band `ALTER TABLE … RENAME COLUMN`. `slug` is simply the column
  name.

```
dbd reset --scope dojo --force
dbd deploy --scope dojo
dbd diff --scope dojo --exit-code     # proof
```

Verified: reset, rebuild, `--exit-code` clean, `tenant_origin` holds exactly
`personal | organization`.

### The lifecycle, and when each command applies

`reconcile` is a **pre-release stopgap**, not a permanent fixture. The cutover is
already encoded in the Makefile and the release workflow:

| | pre-release (today) | post-release |
|---|---|---|
| schema travels as | the DDL tree, applied in place | a versioned snapshot committed by `make bump` |
| command | `dbd reconcile --scope dojo` | `dbd deploy --scope dojo`, migrating v(n) → v(n+1) |
| `reconcile` | the mechanism | **disabled — it errors out** |
| destructive change | just reset | a deliberate, written migration |

`dbd release` is run **once**, at the first public release: it sets
`released: true`, disables `reconcile`, and writes the baseline snapshot. From
that point `make bump`'s snapshot step is load-bearing — a release whose DDL
changed without one deploys nothing while reporting success, which is why the
Makefile makes a missing dbd a hard error post-release rather than a skipped
step.

### Two behaviours worth carrying forward

Neither is a live problem, but both cost time to discover.

1. **dbd cannot express a column rename.** It plans `DROP` + `ADD`, which empties
   the column. It refuses without `--allow-destructive` — the trap announces
   itself — but that flag on a rename is exactly how the data goes. Pre-release
   this is moot (reset); post-release a rename is a written migration anyway.
2. **A data migration and its seed must move together.** Ordering is apply
   (hooks) → import, so the seed has the last word: a hook that rewrites a key
   without the matching seed edit has the old row re-inserted on every deploy.
   Observed here as two `global-dojo` tenants before the seed was updated. This
   one survives the cutover — it is a property of the import phase, not of
   `reconcile`.

**Corrected from an earlier draft:** the finding that `reconcile` skips
`apply.after` hooks was framed as a lurking risk. It is narrower than that.
Post-release `reconcile` is disabled outright and the path is snapshot →
`deploy`, which does run hooks. It is a hand-run hazard during pre-release only,
and CI never had it — the workflow already runs `deploy` after `reconcile`.

---

# Part VIII — Review against the code, and the four decisions that unblock phase 1

Parts I–VII were written against the DDL. This part was written against the
**running code and the live database**, and it changes four things. Where Part
VIII and any earlier part disagree, **Part VIII wins**.

Everything below was verified on the local Supabase (`127.0.0.1:54322`) on
2026-08-27, not inferred from the source.

## VIII.1 F9 — BLOCKING. The sync plan is a user question asked at a tenant address

§V.4 specifies `GET /v1/t/{tenant}/sync/plan`, and in the same breath lists
`unmapped` as a `denied[].reason`: *"`no_seat`, `not_subscribed` and `unmapped`
are three different problems."*

**A repo that maps to no tenant cannot appear in a per-tenant response.** The
endpoint contradicts its own payload, and the contradiction is load-bearing: the
daemon cannot address the plan by tenant, because *which tenant a repo belongs
to is the very thing the dōjō is being asked* (§II.6 puts the mapping dōjō-side,
behind `tenant_connections`). Asking per tenant requires the answer first.

**Resolved: both calls move to the user plane, and mapping is separated from
entitlement.**

```
POST /v1/you/repositories        — register + map (identity)
  → { repos: [ { repo_key, remote_url, name } ] }     the daemon's SHARED repos
  ← { mapped:   [ { repo_key, tenant } ],
      unmapped: [ "gitlab.com/acme/x" ] }             no connection → stays local

GET  /v1/you/sync/plan           — entitlement filter over what is registered
  ← { allowed: [ { repo_key, tenant, repo_id } ],
      denied:  [ { repo_key, tenant, reason } ] }
```

Why this shape and not a single `POST /v1/you/sync/plan` carrying the repo list:

- **§V.4's own division survives.** *"Repo identity is registered separately on
  connect; the plan is the entitlement filter applied on top."* One call would
  collapse identity and entitlement back together — the thing the plan design
  was for.
- **`dojo.repositories` stops being an orphan.** It is currently referenced
  **nowhere** in `dojo/src` — zero reads, zero writes, zero rows. Registration is
  what makes the table real, and it is the table the plan reads.
- **The plan stays a GET**, so it is a read in the HTTP sense as well as the
  semantic one, and the daemon does not re-ship its whole repo list every cycle.
- **`tenant_id NOT NULL` is respected.** `dojo.repositories` is keyed
  `(tenant_id, repo_key)` with a NOT NULL tenant. An unmapped repo therefore has
  no row it *can* occupy — which is exactly §II.6's rule ("unmapped, not
  personal — it stays local-only until its org is connected"). Registration
  reports it and writes nothing; the plan never sees it. **`unmapped` is
  consequently a registration outcome, not a `denied[]` reason** — correcting
  §V.4's list to `not_subscribed | no_seat | subscription_expired | unclaimed`.

Everything Part V claimed for the tenant-scoped plan still holds: no cache, no
TTL, allow-list not permission-check, offline degrades to no-sync, and the sensei
schema delta across all phases remains one display-only column.

## VIII.2 F10 — BLOCKING. Two identity grains, and the spec picks neither

§II.7 step 1 says *"upsert `dojo.identities` (provider, subject)"*; Part I
Scenario 6 references `identities.user_id`. Neither exists. The live table is:

```
dojo.identities ( id, principal_id → dojo.principals(id), provider, subject, … )
dojo.principals ( id, auth_user_id unique → the Supabase login, re-pointable )
```

Meanwhile `dojo.memberships.user_id` and `dojo.projects.user_id` are both
documented as *the raw Supabase auth subject* — which `principals.ddl` expressly
forbids: *"nothing else should reference auth.users directly."* The schema holds
two grains and the spec never says which one `ensureProvisioned(userId, …)`
takes.

**Resolved: the principal is the grain, everywhere.**

```
auth.users.id ──▶ principals.auth_user_id
                     ├─▶ identities.principal_id
                     ├─▶ memberships.user_id      (= principal id)
                     └─▶ projects.user_id         (= principal id)
```

This is what `principals` was built for — an accidentally-merged Supabase account
stays recoverable, because nothing downstream referenced the login.

**The change is contained, because `resolveCaller` is a chokepoint.** It maps
`JWT sub → principals.auth_user_id → principal id` and returns it as `userId`;
every call site consuming `caller.userId` is then correct unchanged. Measured:
52 `user_id` references across 16 server files, of which only **three** are
client-visible and need the console to send principal ids —
`POST …/members` (`body.user_id`), `POST …/identities` (`body.user_id`), and
`PATCH …/members/{userId}/role` (`params.userId`). All three already round-trip
whatever `listMembers` returned, so they stay consistent by construction.

**One schema consequence, and it fails silently if missed.** `dojo.projects`
grants `select` to `authenticated` under

```sql
using (user_id = (select auth.uid()))
```

If `projects.user_id` becomes a principal id, that policy matches **nothing** and
a client-direct read returns zero rows — honest-empty masking a break, which the
no-fabrication rule forbids. The policy must resolve the principal:

```sql
using (user_id = (select p.id from dojo.principals p
                   where p.auth_user_id = (select auth.uid())))
```

The Worker uses `service_role` and bypasses RLS, so **the app would not notice.**
That is precisely why it is written down here.

## VIII.3 F11 — BLOCKING. There is no web sign-in callback to wire into

§II.7 lists "web sign-in callback" as a provisioning caller. The dōjō has no such
route: kavach owns the callback (`kavach.config.js` → `routes.session:
'/auth/session'`, served internally by `kavach.handle`), and
`src/routes/+layout.server.ts` only reflects `locals.session` onto the page.

**Resolved: `POST /v1/you/provision`**, called by the client once immediately
after sign-in. An explicit endpoint on the plane that already exists, testable
without a browser, and it keeps provisioning out of a layout load that runs on
every navigation. The three callers of §II.7 become:

| caller | when |
|---|---|
| `POST /v1/you/provision` | client-side, immediately after web sign-in |
| `POST /v1/auth/cli/token` | when the daemon completes device auth |
| `POST /v1/you/github/sync` | explicit re-sync from the console |

`POST /v1/auth/cli/token` must **not** reshape its response — its own comment
says re-modelling the payload is how `provider_token` gets quietly dropped. It
parses a clone of the upstream body for `{ user.id, provider_token }`, provisions,
and returns the original text verbatim.

## VIII.4 F12 — BLOCKING. Two shipped paths are dead, and the suite is green

`bun run test` → **exit 0, 123 files, 1328 tests**, with two production paths
broken against the live schema. Both confirmed by running their exact statements:

| path | error | since |
|---|---|---|
| `createDojo` → `POST /v1/you/dojos` | `column "org" of relation "tenants" does not exist` | `37ca9fab` (this slice) |
| every `dojo.identities` read/write | `column "user_id" does not exist` | `75565304` (pre-existing) |

`createDojo` also inserts `origin: 'org'`, which is no longer a `tenant_origin`
label, and derives `dojo_url` from the retired `org/{slug}` key. **This is issue
#117's own acceptance criterion** — the feature the slice is filed under is dead.

The identities breakage reaches further than provisioning: the four
`/v1/t/…/identities` routes, plus `resolveDisplayNames` → `listMembers` → the
members console screen, plus `incidents-data.ts`.

**Why the tests passed.** The specs stub the Supabase client and assert the
payload the code *sends* — `admin-data.spec.ts:349` asserts
`{ key: 'org/acme', origin: 'org', org: 'acme' }`, a shape the database rejects.
No dōjō test touches a real Postgres, so **no dōjō test can fail on schema
drift.** `ensureProvisioned` tested the same way would be green and
non-functional in exactly this manner.

**Resolved:** both are repaired inside this slice, failing-test-first, and phase 1
adds **at least one test that executes against the live Postgres** so that the
next schema change has something that can go red. That test is a phase-1
deliverable, not a follow-up.

## VIII.5 Reuse — the normaliser already exists, in Rust

`crates/senseid/src/db/pg_store/repo_key.rs::normalize_repo_key` already collapses
SSH / HTTPS / `ssh://` / `git://`, strips userinfo, port and `.git`, lowercases,
and yields `host/org/repo`. It is what writes `sensei.repositories.repo_key`.

**The dōjō must not re-implement it.** Registration (§VIII.1) receives the
already-normalised `repo_key`, so the dōjō's job is the narrower, genuinely new
`repo_key → (provider, org_slug)`, which is a host-and-path mapping rather than
URL parsing:

| forge | repo_key | provider | org |
|---|---|---|---|
| github | `github.com/acme/api` | `github` | `acme` (seg 1) |
| gitlab | `gitlab.com/acme/sub/api` | `gitlab` | `acme` (seg 1, top-level group) |
| bitbucket | `bitbucket.org/acme/api` | `bitbucket` | `acme` (seg 1) |
| azure_devops | `dev.azure.com/acme/proj/_git/api` | `azure_devops` | `acme` (seg 1) |
| azure_devops | `vs-ssh.visualstudio.com/v3/acme/proj/api` | `azure_devops` | `acme` (**seg 2**, after `v3`) |

The second Azure form is why "the org is the first segment" is wrong as a general
rule, and why this is a typed per-provider mapping rather than a `split('/')[1]`.
An unrecognised host yields no provider — and therefore `unmapped`, never a
guess.

## VIII.6 Smaller corrections

- **`dojo.tenants` comments are stale.** `key` still documents
  `"<origin>/<org>[/<dojo>]"` with the examples `github/sensei-hq` /
  `org/global-dojo`, and `origin` still reads *"github (backed by a GitHub org
  identity) or org (custom-registered name)"*. Both describe the retired model —
  and they are what an implementer reads first.
- **`syncGithubMemberships` is superseded, not patched.** It builds
  `github/{login}` keys, which no longer exist, and its contract ("never invents
  a tenant") is the hole this slice closes. It is replaced by the
  `tenant_connections` lookup inside `ensureProvisioned`.
- **`dojo.auth_method` has no generic `oauth`**, and `identities.provider` is
  typed on it. Phase 1 therefore accepts `provider = 'github'` only, despite
  `ensureProvisioned`'s multi-forge signature. Phase 2 adds the label (§V.1).
- **The kavach double-resolve bug is fixed** (1.1.0, pin verified installed), so
  `/v1` POST bodies arrive intact. Both new POST callers depend on this.

## VIII.7 Phase 1, corrected

| # | deliverable | status |
|---|---|---|
| 1 | RLS fix on `dojo.projects` (principal-resolving policy, §VIII.2) | ✅ `bb994b6a` |
| 2 | `resolveCaller` maps `sub → principal id`; `ensureProvisioned(userId, forgeToken, provider)` writes `principals` → `identities` → personal tenant → org tenants + `tenant_connections` → memberships, idempotently | ✅ `dd6917f7`, `2eda4236` |
| 3 | Repair `createDojo` (`organization/{slug}`, `slug` column, `dojo_url`) — issue #117's AC | ✅ `11ebd83e` |
| 4 | Repair every `dojo.identities` path onto `principal_id` | ✅ `5cbe4d4d` |
| 5 | `POST /v1/you/provision`, and `POST /v1/auth/cli/token` provisioning without reshaping its response | ✅ `b344da86` |
| 6 | `POST /v1/you/repositories` — `repo_key → (provider, org)` → `tenant_connections` → tenant; upsert `dojo.repositories`; report `unmapped` | ✅ `acda527a` |
| 7 | `GET /v1/you/sync/plan` — everything registered `allowed` in phase 1 | ✅ `acda527a` |
| 8 | At least one live-Postgres test, so schema drift can go red | ✅ `bb994b6a` (5 files by `acda527a`) |

Items 1–4 were prerequisites: provisioning writes through exactly the paths that
were broken.

### What item 1 turned out to be

The spec named one surface. There were **three**, all making the same mistake:
`dojo.projects`'s policy, `dojo.owns_membership` (which backs the
`relay_sessions` / `relay_inbox` / `relay_segments` policies, so all three were
silently empty), and `can_read_repository_metric`'s admin branch, which compared
`memberships.user_id` to `principals.auth_user_id` and therefore never matched —
a tenant admin quietly lost per-user metrics. All three now call one
`dojo.current_principal_id()`.

### What item 4 turned out to be

The dropped `tenant_id` filter was also, incidentally, the **tenant isolation**
on those routes. Removing it without replacement would have let an admin of one
tenant read, rewrite and delete the identities of people in another. It is now an
explicit membership check, and update/delete answer `404 no such identity` for
both the missing and the wrong-tenant case so tenant A cannot probe which ids
exist in tenant B.

### VERIFIED against a real GitHub sign-in — 2026-08-27

The last unobserved assumption of the design (§IV.8) is now observed, and it
holds: **Supabase does return `provider_token` on the exchange, and the dōjō can
read the caller's orgs with it.** A real sign-in produced, from nothing:

| table | row |
|---|---|
| `dojo.principals` | one, pointing at the Supabase login |
| `dojo.identities` | `github_oauth` / subject `293381742` — GitHub's stable USER id, not the login |
| `dojo.tenants` | `personal/sensei-hq-org` (slugged from the GitHub login) **and** `organization/sensei-hq` |
| `dojo.memberships` | `admin` on both, `authenticated_via = github_oauth` |
| `dojo.tenant_connections` | `github` / external_id `276295035` — the stable ORG id — `verified_at` set |

Idempotence verified on that real data: two further session syncs left every
count identical, with no duplicate tenant key, membership or connection.

The honest-failure paths were exercised separately against the live endpoint,
and the three outcomes stay distinguishable rather than collapsing into "nothing
happened":

```
no token         → { synced: false, reason: "no_forge_token",    personal: {…} }
invalid token    → { synced: false, reason: "forge_unreachable", personal: {…} }
valid token      → { synced: true,  personal: {…}, tenants: [organization/sensei-hq] }
```

In both failure cases the personal dōjō is still returned — D1 does not depend
on any forge — and no org tenant is invented from a read that did not succeed.

**How the token reaches the server.** Not through the session cookie:
`setCookieFromSession` keeps only `access_token`/`refresh_token`, so
`locals.session.provider_token` is structurally always null and the web path
could never have provisioned an org. kavach gained an `onSessionSync` server
hook (jerrythomas/kavach `040d34c`) which hands the app the INCOMING provider
session — the payload the browser already POSTs to `/auth/session`. Nothing
extra is persisted and nothing new crosses the wire; the token was already
arriving and being discarded. Persisting it in the cookie was considered and
rejected: a `read:org` token replayed on every request for the session's
lifetime, to serve a need that lasts one call.

**Carry forward:** `node_modules/kavach` is patched locally with that commit.
A published `1.1.1` and a repin are required before this deploys anywhere.
