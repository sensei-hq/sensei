Feature: Dōjō Consoles — the org context role surfaces (built; not yet pinned)

  # resolved: T-2
  NO TEST
  Scenario: the org context /org/[slug] exists with its own nav pane
    Given a user with an org membership
    When they click the org in My Dōjōs
    Then they land on /org/[slug] with a nav pane for workspaces, governance, and role surfaces
    Built — /org/[slug]/[section], nav from navForOrg in dojo/src/lib/nav.ts.

  # resolved: T-2
  NO TEST
  Scenario: a maintainer sees governance authoring in the org context
    Given a user with maintainer role
    When they open /org/[slug]/governance
    Then they can author stance (autonomy, sharing, review, anonymization) + rules/skills/agents/commands
    The Govern group is gated at maintainer (navForOrg); the authoring surface itself is unbuilt (#91).

  # resolved: T-2
  NO TEST
  Scenario: a maintainer sees triage and approvals in the org context
    Given a user with maintainer role
    When they open /org/[slug]/triage
    Then they see findings awaiting review with auto-approve scores and can decide
    Built — the Govern group is gated at maintainer (navForOrg).

  # resolved: T-2
  NO TEST
  Scenario: a maintainer sees knowledge management in the org context
    Given a user with maintainer role
    When they open /org/[slug]/knowledge
    Then they can manage the org's knowledge base
    Built — the Govern group is gated at maintainer (navForOrg).

  # resolved: T-2
  NO TEST
  Scenario: an admin sees members, identities, policies, and billing in the org context
    Given a user with admin role
    When they open the org context
    Then they see /members, /identities, /policies, /billing
    Built — the Admin group is gated at admin (navForOrg).

  # resolved: T-2
  NO TEST
  Scenario: a dōjō has no Clients section (D34)
    Given a user with any role
    When they open the org context
    Then there is no /engagements, /incidents or /clientaudit section
    Built — nav.spec.ts and shell.spec.ts assert the absence (#307 R7).

  # resolved: T-2
  NO TEST
  Scenario: a contributor sees read-mostly surfaces in the org context
    Given a user with contributor role
    When they open the org context
    Then they can read workspaces in jurisdiction, preview a workspace's constitution, and view members
    Built — the Overview group has no role floor (navForOrg).

  # gap: #307 A4
  NO TEST
  Scenario: a workspace drill shows which ladder levels built its constitution
    Given a workspace in the org's jurisdiction
    When a user clicks it in /org/[slug]
    Then /org/[slug]/projects/[id] shows which ladder levels contributed, conflicts settled, and locks
    Not built — the org workspace drill-in still reads fixtures (#307 A4).

  # resolved: T-2
  NO TEST
  Scenario: a user with multiple roles sees the union of their surfaces
    Given a user who is both maintainer and admin in an org
    When they open the org context
    Then they see both governance and roles/policies surfaces
    Built — the ladder is additive, so a higher role sees every lower role's groups (navForOrg).

  # gap: T-2
  NO TEST
  Scenario: the personal landing is work-first for members
    Given a member of a Dōjō
    When they sign in
    Then they land on the personal view with "your work" primary and My Dōjōs secondary
    Currently a member still lands on the old management Overview.