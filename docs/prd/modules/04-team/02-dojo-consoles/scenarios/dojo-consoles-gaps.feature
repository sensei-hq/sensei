Feature: Dōjō Consoles — the unbuilt org context role surfaces

  # gap: T-2
  NO TEST
  Scenario: the org context /org/[slug] exists with its own nav pane
    Given a user with an org membership
    When they click the org in My Dōjōs
    Then they land on /org/[slug] with a nav pane for projects, governance, and role surfaces
    This is unbuilt — org screens currently live flat under /console/*.

  # gap: T-2
  NO TEST
  Scenario: a maintainer sees governance authoring in the org context
    Given a user with maintainer role
    When they open /org/[slug]/governance
    Then they can author stance (autonomy, sharing, review, anonymization) + rules/skills/agents/commands
    This is unbuilt — role-gating not wired.

  # gap: T-2
  NO TEST
  Scenario: a maintainer sees triage and approvals in the org context
    Given a user with maintainer role
    When they open /org/[slug]/triage
    Then they see findings awaiting review with auto-approve scores and can decide
    This is unbuilt — role-gating not wired.

  # gap: T-2
  NO TEST
  Scenario: a maintainer sees knowledge management in the org context
    Given a user with maintainer role
    When they open /org/[slug]/knowledge
    Then they can manage the org's knowledge base
    This is unbuilt — role-gating not wired.

  # gap: T-2
  NO TEST
  Scenario: an admin sees members, identities, policies, and billing in the org context
    Given a user with admin role
    When they open the org context
    Then they see /members, /identities, /policies, /billing
    This is unbuilt — role-gating not wired.

  # gap: T-2
  NO TEST
  Scenario: a lead sees engagements, incidents, and audit in the org context
    Given a user with lead role
    When they open the org context
    Then they see /engagements, /incidents, /audit
    This is unbuilt — role-gating not wired.

  # gap: T-2
  NO TEST
  Scenario: a developer sees read-mostly surfaces in the org context
    Given a user with developer role
    When they open the org context
    Then they can read projects in jurisdiction, preview a project's constitution, and view members
    This is unbuilt — role-gating not wired.

  # gap: T-2
  NO TEST
  Scenario: a project drill shows which ladder levels built its constitution
    Given a project in the org's jurisdiction
    When a user clicks it in /org/[slug]
    Then /org/[slug]/projects/[id] shows which ladder levels contributed, conflicts settled, and locks
    This is unbuilt — the /console/preview demo is a placeholder.

  # gap: T-2
  NO TEST
  Scenario: a user with multiple roles sees the union of their surfaces
    Given a user who is both maintainer and admin in an org
    When they open the org context
    Then they see both governance and roles/policies surfaces
    This is unbuilt — additive grants not wired.

  # gap: T-2
  NO TEST
  Scenario: the personal landing is work-first for members
    Given a member of a Dōjō
    When they sign in
    Then they land on the personal view with "your work" primary and My Dōjōs secondary
    Currently a member still lands on the old management Overview.