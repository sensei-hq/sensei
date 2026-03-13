---
id: identity-access-pricing
type: feature
---

# Identity, Access & Pricing

> Sensei is free for open source, affordable for teams, and your code never leaves your control.

Sensei's value is measurable: a 70–80% reduction in token usage turns a $100/month Claude subscription into a $20 one. That's $80 saved per developer per month. At $5/seat/month, sensei pays for itself 16x over. Open source projects are always free — no gates, no expiry. Local (self-hosted) mode is always free — Supabase runs on your machine, your data never leaves.

---

## Features

### Authentication

Sensei supports multiple OAuth providers. GitHub is the primary auth option (natural fit — developers already use it, repos live there). Magic link is available for enterprise users and teams not on GitHub. Additional OAuth providers (Google, GitLab, Bitbucket, Azure) are supported via the same auth layer.

```gherkin
Feature: Authentication

  Scenario: Developer signs in with GitHub
    Given a user visits the sensei web app
    When they click "Sign in with GitHub"
    Then they are redirected to GitHub OAuth
    And on return, an authenticated session is created
    And their GitHub username and avatar are recorded

  Scenario: Enterprise user signs in with magic link
    Given a user enters their work email on the login page
    When they click "Send magic link"
    Then a login link is sent to their email
    And clicking it creates an authenticated session
    And they are assigned to their team based on email domain

  Scenario: User signs in with alternative OAuth provider
    Given a user clicks "Sign in with Google"
    Then they are redirected to Google OAuth
    And on return an authenticated session is created
    And they can link this account to an existing sensei profile

  Scenario: Session expires
    Given a user's session has expired
    When they make a request to sensei
    Then they receive an authentication error
    And are redirected to the login page to re-authenticate
```

---

### Multi-Tenancy & Repo Isolation

Each team's data is fully isolated. A private repo's symbols, sessions, analytics, and snapshots are only visible to team members. Public (open source) repos are readable by anyone — no account required to browse their index.

```gherkin
Feature: Repo Isolation

  Scenario: Private repo data is invisible to outsiders
    Given user A owns a private repo indexed in sensei
    And user B is not a member of user A's team
    When user B queries the sensei API for that repo's symbols
    Then they receive a 404 (not found, not 403)
    And no data about the repo is leaked

  Scenario: Public repo is accessible to all
    Given an open source repo is indexed and marked public
    When any unauthenticated user queries its index
    Then they receive the repo's symbols, docs, and context packs
    And no authentication is required

  Scenario: Team member can access shared private repo
    Given user B is a member of team T
    And a private repo is registered under team T
    When user B queries that repo via sensei
    Then they receive full access to its index and analytics

  Scenario: User leaves a team
    Given user B is removed from team T by an owner
    When user B attempts to access team T's private repos
    Then all requests return 404
    And user B's active sessions on those repos are invalidated
```

---

### Teams & Authorization

Repos are owned by a team (or an individual). Teams have members with roles: owner, member, viewer. Owners manage billing, repo registration, and member invites. Members can index, query, and use all features. Viewers can query but not modify config or trigger reindex.

```gherkin
Feature: Team Authorization

  Scenario: Owner invites a member
    Given a team owner is on the team settings page
    When they enter a GitHub username or email and select role "member"
    Then an invite is sent
    And on acceptance, the user joins the team with member-level access

  Scenario: Member triggers a reindex
    Given a user with member role on team T
    When they run sensei reindex on a team repo
    Then the reindex runs and results are stored under team T's namespace

  Scenario: Viewer cannot trigger reindex
    Given a user with viewer role
    When they attempt to call the reindex MCP tool
    Then they receive a permission denied error
    And no indexing occurs

  Scenario: Repo registered under a team
    Given a team owner runs sensei init on a new repo
    Then the repo is registered under the team
    And access controls ensure only team members can access it
```

---

### GitHub App Integration

The GitHub App is an optional addon that enables richer integration: automatic repo detection, webhook-driven reindex on push, and drift detection triggered on PR open. Developers authorize the app once; sensei sees which repos they have access to and can register them automatically.

```gherkin
Feature: GitHub App Integration

  Scenario: User installs the GitHub App
    Given a user clicks "Connect GitHub App" in sensei settings
    When they authorize the app on GitHub
    Then sensei can list all repos the user has access to
    And unregistered repos are shown as available to add

  Scenario: Auto-reindex on push
    Given a repo is registered in sensei with GitHub App connected
    When a push is made to the main branch
    Then GitHub sends a push webhook to sensei
    And sensei triggers an incremental reindex for changed files
    And updated symbols are available within 60 seconds

  Scenario: Drift check triggered on PR
    Given a PR is opened that modifies code files
    When the GitHub App receives the pull_request event
    Then sensei runs check_drift for the changed files
    And if drift is detected, posts a PR comment listing drifted doc pairs
    And the PR check status is set to "warning" if drift exists

  Scenario: User without GitHub App uses sensei
    Given a user has not installed the GitHub App
    When they run sensei init and sensei reindex manually
    Then all features work normally
    And the GitHub App is surfaced as an optional enhancement
```

---

### Pricing Model

Sensei's pricing is anchored to its value: token reduction that saves developers real money on AI subscriptions. The business case is clear — if sensei reduces Claude usage by 70%, a $100/month subscription becomes $20. Sensei at $5/seat costs less than one coffee per month and pays for itself immediately.

**Local (self-hosted):** Always free. Supabase runs on your machine. No data leaves your environment. Full feature set.

**Open Source:** Always free on cloud. Public repos indexed, full analytics, all features. No time limit, no feature gates. Open source maintainers can support development via GitHub Sponsors, Patreon, or Buy Me a Coffee — entirely optional.

**Pro ($5/seat/month):** Private repos, team access, full analytics, FTR scoring, quality dashboard, external doc retrieval, custom lib indexing, GitHub App integration.

**Enterprise (custom):** SSO/SAML, dedicated hosted instance, SLA, priority support.

```gherkin
Feature: Pricing Enforcement

  Scenario: Open source repo is always free
    Given a repo is public on GitHub and marked as open source
    When it is registered in sensei cloud
    Then no subscription is required
    And all features including analytics and FTR scoring are available

  Scenario: Private repo requires Pro subscription
    Given a user registers a private repo
    And their team has no active subscription
    When they attempt to run sensei init
    Then they are shown the Pro plan pricing and a 14-day free trial offer
    And the trial starts automatically on confirmation

  Scenario: Developer calculates their ROI
    Given a developer uses $100/month of Claude tokens
    When sensei reduces usage by 75%
    Then their effective Claude cost becomes $25/month
    And the savings of $75/month are shown on the dashboard
    And the $5/month sensei cost yields a net saving of $70/month per seat

  Scenario: Open source developer supports via donation
    Given an open source developer benefits from sensei
    When they visit the support page
    Then they see options: GitHub Sponsors, Patreon, Buy Me a Coffee
    And all donation amounts are accepted with no minimum
    And their account is marked as a supporter with no functional changes
```

---

### Data Privacy, Self-Hosting & Telemetry Control

Sensei's architecture ensures code never has to leave your machine. Local deployment is a first-class option for compliance-sensitive teams. For users on cloud, raw source code is never stored — only extracted symbols, embeddings, and session metadata. Anonymous telemetry contribution is opt-in for private repos and opt-out for open source repos; all contributed data is de-identified (repo names, file paths, user IDs stripped) before aggregation. Contributed data powers the aggregate benchmarks and developer coaching engine that benefit all users.

```gherkin
Feature: Data Privacy and Telemetry Control

  Scenario: Local mode keeps all data on-device
    Given a developer runs sensei in local mode
    When they index a repo and use context_pack
    Then all data is stored in the local backend instance
    And no data is sent to sensei.dev or any external service
    And the developer can verify this via network inspection

  Scenario: Cloud mode stores only index artifacts, not source code
    Given a developer uses sensei cloud
    Then only extracted symbols and session metadata are stored
    And raw source code files are never uploaded or retained
    And the privacy policy documents the exact fields retained

  Scenario: Private repo developer opts in to anonymous telemetry
    Given a developer with a private repo
    When they enable "Contribute anonymous telemetry" in settings
    Then future session metrics (token counts, FTR scores, task types) are included in aggregate benchmarks
    And repo names, file paths, and user IDs are stripped before any data leaves the device
    And a preview of exactly what will be contributed is shown before confirmation

  Scenario: Open source repo opts out of telemetry
    Given an open source repo contributing telemetry by default
    When the developer toggles off "Contribute anonymous telemetry"
    Then no further metrics from that repo are included in aggregate calculations
    And previously contributed data (already anonymised) remains in aggregate totals

  Scenario: Developer verifies their telemetry contribution
    Given a developer who has opted in to anonymous telemetry
    When they open the Telemetry Audit view in settings
    Then they see a log of the last 30 days of contributed data
    And each entry shows only the anonymised fields: task type, turn count, token count, FTR score, stack, and agent name
    And they can delete their contribution history at any time

  Scenario: Team data deletion
    Given a team owner requests data deletion
    When they confirm deletion in team settings
    Then all repos, symbols, sessions, and analytics for that team are deleted
    And deletion is confirmed via email within 24 hours
```
