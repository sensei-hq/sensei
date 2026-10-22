Feature: Dōjō Core — the org context, a dōjō from a git integration, and rule sync

  # resolved: T-2
  NO TEST
  Scenario: the org context /org/[slug] has its own nav with workspaces, ladder config, and role surfaces
    Given a user with an org membership
    When they click the org in "My Dōjōs"
    Then they land on /org/[slug] with its own nav pane showing workspaces, governance ladder, and role-scoped surfaces
    Built (T-2 resolved) — /org/[slug]/[section], nav from navForOrg in dojo/src/lib/nav.ts.

  # resolved: T-2
  NO TEST
  Scenario: a maintainer sees the governance authoring surface in the org context
    Given a user with maintainer role in an org
    When they open the org's Govern sections
    Then they can author rules/stance on the ladder, triage, approve, and manage knowledge
    Built — the Govern group is gated at maintainer (navForOrg).

  # resolved: T-2
  NO TEST
  Scenario: an admin sees the members/identities/policies/billing surfaces in the org context
    Given a user with admin role in an org
    When they open the org context
    Then they see members, identities/SSO, scopes & precedence, and billing
    Built — the Admin group is gated at admin (navForOrg).

  # resolved: T-2
  NO TEST
  Scenario: no role sees a Clients section in the org context (D34)
    Given a user with lead role in an org
    When they open the org context
    Then they see the Overview and Govern groups, as a maintainer does
    And no Engagements, Incidents or Client audit section appears for any role
    Built — nav.spec.ts and shell.spec.ts assert the absence (#307 R7).

  # resolved: T-2
  NO TEST
  Scenario: a contributor sees read-mostly surfaces in the org context
    Given a user with contributor role in an org
    When they open the org context
    Then they can read workspaces, preview a workspace's constitution, and view members
    Built — the Overview group has no role floor (navForOrg).

  # gap: #308 (D20)
  NO TEST
  Scenario: a dōjō is imported from a git integration and starts on the free tier
    Given a signed-in admin of a GitHub or Azure DevOps organization
    When they import the organization as a dōjō
    Then the dōjō exists on the free tier with a starter constitution (personal ladder + company rung)
    And the plan is chosen later in the org's billing section, not at import
    Not built — tracked by #308. The hand-made create flow is retired (D11/D20, R1).

  # gap: F-3
  NO TEST
  Scenario: Dōjō sync works end-to-end
    Given a local sensei with a Dōjō membership
    When it contributes a finding
    Then the finding is accumulated, triaged, approved, distributed, and received downstream
    Works against the local dōjō; production is unverified.

  # gap: F-3
  NO TEST
  Scenario: a Dōjō-authored rule appears in the member's resolved set
    Given an org that authored a mandatory rule at its scope
    When a member whose repo is bound to that org calls get_rules
    Then the org's rule is in the resolved set
    Not pinned yet — the sync path works against the local dōjō, production is unverified.

  NO TEST
  Scenario: the personal landing is work-first for members too
    Given a member of a Dōjō
    When they sign in
    Then they land on the personal view (your work) with My Dōjōs section showing orgs + role
    Currently a member still lands on the old management Overview.