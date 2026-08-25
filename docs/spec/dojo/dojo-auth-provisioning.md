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
