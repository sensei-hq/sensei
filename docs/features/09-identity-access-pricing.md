---
id: identity-access-pricing
type: feature
---

# Identity, Access & Pricing

> Sensei is free for open source, affordable for teams, and your code never leaves your control.

Sensei's value is measurable: a significant reduction in token usage directly lowers what developers spend on AI subscriptions, and at sensei's per-seat price it pays for itself many times over. Open source projects are always free — no gates, no expiry. Local (self-hosted) mode is always free — everything runs on your machine, your data never leaves.

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
    When user B queries sensei for that repo's symbols
    Then they receive a not-found response (no data leaked, no existence confirmed)
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
    Then all requests return not-found responses
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
    When they attempt to trigger a reindex
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
    When the GitHub App receives the pull request event
    Then sensei runs a drift check for the changed files
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

Sensei's pricing is anchored to its value: token reduction that saves developers real money on AI subscriptions. The business case is clear — if sensei significantly reduces token usage, the subscription savings alone cover the cost many times over.

**Local (self-hosted):** Always free. Everything runs on your machine. No data leaves your environment. Full feature set.

**Open Source:** Always free on cloud. Public repos indexed, full analytics, all features. No time limit, no feature gates. Open source maintainers can support development via optional donations — entirely optional.

**Pro:** Private repos, team access, full analytics, FTR scoring, quality dashboard, external doc retrieval, custom lib indexing, GitHub App integration.

**Enterprise:** SSO/SAML, dedicated hosted instance, SLA, priority support.

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
    When they attempt to initialise sensei
    Then they are shown the Pro plan details and a free trial offer
    And the trial starts automatically on confirmation

  Scenario: Developer calculates their ROI
    Given a developer has measurable AI token spend per month
    When sensei reduces usage significantly
    Then the effective AI cost reduction is shown on the dashboard
    And the net saving after sensei's cost is displayed per seat

  Scenario: Open source developer supports via donation
    Given an open source developer benefits from sensei
    When they visit the support page
    Then they see available donation options
    And all donation amounts are accepted with no minimum
    And their account is marked as a supporter with no functional changes
```

---

### Data Privacy, Self-Hosting & Telemetry Control

Sensei's architecture ensures code never has to leave your machine. Local deployment is a first-class option for compliance-sensitive teams. For users on cloud, raw source code is never stored — only the index artifacts and session metadata needed to deliver context. Anonymous telemetry contribution is opt-in for private repos and opt-out for open source repos; all contributed data is de-identified (repo names, file paths, user IDs stripped) before aggregation. Contributed data powers the aggregate benchmarks and developer coaching engine that benefit all users.

```gherkin
Feature: Data Privacy and Telemetry Control

  Scenario: Local mode keeps all data on-device
    Given a developer runs sensei in local mode
    When they index a repo and request context
    Then all data is stored in the local backend instance
    And no data is sent to any external service
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

---

### Organization & Team Hierarchy

Users belong to organizations (many-to-many via `org_members`). Organizations contain teams; teams group repos and members for access control.

- **Organizations** — top-level namespace, inferred from the repo's remote URL where possible (see Repo Identity below)
- **Teams** — scoped within an organization; unique per org; used to grant access to a set of repos
- **`org_members`** — join table linking users to organizations with a role (`owner`, `member`, `viewer`)
- **Automatic inference** — if a repo has a remote URL, the org and user are inferred from it (e.g. GitHub: `github.com/{org}/{repo}` → org = `{org}`)
- **Manual fallback** — if the org cannot be inferred (self-hosted Git, bare remote, no remote), the developer configures org and team manually in settings

```gherkin
Feature: Organization & Team Hierarchy

  Scenario: Org is inferred from GitHub remote URL
    Given a repo with remote URL github.com/acme/my-service
    When the developer runs sensei init
    Then the org is set to "acme" automatically
    And the repo is registered under the "acme" organization

  Scenario: Org cannot be inferred from remote URL
    Given a repo hosted on a self-hosted Git server with no recognised URL pattern
    When the developer runs sensei init
    Then sensei prompts the developer to specify their organization name manually
    And the entered value is saved to .sensei/config.yaml as the org field

  Scenario: Team member accesses a team repo
    Given user B is a member of team "backend" in org "acme"
    And the repo "my-service" is registered under team "backend"
    When user B queries sensei for that repo
    Then full access is granted to the repo's index and analytics
```

---

### Repo Identity & Remote URL Adapters

Sensei uses an adapter pattern to parse repo remote URLs and infer org, user, and repo name. Adapters are tried in order; the first match wins. Unknown patterns fall back to a manual prompt.

| Provider | URL Pattern | Org inferred as | Notes |
|---|---|---|---|
| GitHub | `github.com/{org}/{repo}` | `{org}` | Also `git@github.com:{org}/{repo}.git` |
| GitLab.com | `gitlab.com/{group}/{subgroup}/{repo}` | `{group}` | Subgroups are flattened |
| Bitbucket | `bitbucket.org/{workspace}/{repo}` | `{workspace}` | |
| Azure DevOps | `dev.azure.com/{org}/{project}/_git/{repo}` | `{org}` | Also `ssh.dev.azure.com/v3/{org}/...` |
| Self-hosted GitLab | `{configured-base-url}/{group}/{repo}` | `{group}` | Base URL configured in `~/.config/sensei/config.yaml` |
| Unknown | any unrecognised pattern | — | Prompts user to set org manually |

**Privacy default:** all repos are treated as private unless an explicit `public: true` flag is set in `.sensei/config.yaml`. This prevents accidental data exposure when the URL is public but the intended audience is not.

```gherkin
Feature: Repo Identity Adapters

  Scenario: GitHub SSH remote URL is parsed correctly
    Given a repo with remote git@github.com:acme/api.git
    When sensei init runs
    Then the org is parsed as "acme" and the repo name as "api"
    And no manual prompt is shown

  Scenario: Azure DevOps URL is parsed correctly
    Given a repo with remote https://dev.azure.com/contoso/MyProject/_git/my-service
    When sensei init runs
    Then the org is parsed as "contoso" and the repo name as "my-service"

  Scenario: Unknown remote URL prompts for manual org entry
    Given a repo with remote https://git.internal.corp/team/service.git
    And no self-hosted GitLab base URL is configured
    When sensei init runs
    Then the developer is prompted to enter their organization name
    And the value is saved and used for all future requests
```

---

### User Registration & Identity Linking

Developers register on the sensei website, then link their machine to their account using a CLI auth command. All repos configured with sensei on that machine are automatically associated with the registered account.

1. Developer signs up at the sensei website (GitHub OAuth or magic link)
2. Developer runs `sensei auth login` on their machine — opens browser, completes OAuth, generates a device token
3. Token is stored in `~/.config/sensei/credentials.yaml` alongside the Supabase service key
4. All repos with `.sensei/config.yaml` on that machine auto-associate with the user's account
5. Dashboard shows all repos across all machines under the user's account

```gherkin
Feature: User Registration & Identity Linking

  Scenario: Developer links their machine after registration
    Given a developer has registered on the sensei website
    When they run sensei auth login on their machine
    Then a browser window opens for OAuth confirmation
    And on completion, a device token is written to ~/.config/sensei/credentials.yaml
    And the CLI confirms the machine is linked to the registered account

  Scenario: Repos auto-associate with the registered account
    Given a developer has completed sensei auth login on their machine
    And multiple repos have .sensei/config.yaml on that machine
    When they open the sensei dashboard
    Then all repos from that machine are listed under their account
    And session analytics for each repo are visible

  Scenario: Developer uses sensei on a second machine
    Given the developer has an existing account with repos linked from machine A
    When they run sensei auth login on machine B
    Then the machine B is linked to the same account
    And repos initialised on machine B are added to the existing account view in the dashboard
    And repos from machine A remain accessible
```
