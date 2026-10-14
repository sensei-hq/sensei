Feature: Dōjō Core — the unbuilt org context and create-Dōjō flow

  # gap: T-2
  NO TEST
  Scenario: the org context /org/[slug] has its own nav with projects, ladder config, and role surfaces
    Given a user with an org membership
    When they click the org in "My Dōjōs"
    Then they land on /org/[slug] with its own nav pane showing projects, governance ladder, and role-scoped surfaces
    This is unbuilt — org screens currently live flat under /console/*.

  # gap: T-2
  NO TEST
  Scenario: a maintainer sees the governance authoring surface in the org context
    Given a user with maintainer role in an org
    When they open /org/[slug]/governance
    Then they can author rules/stance on the ladder, triage, approve, and manage knowledge
    This is unbuilt — role-gating not wired.

  # gap: T-2
  NO TEST
  Scenario: an admin sees the members/identities/policies/billing surfaces in the org context
    Given a user with admin role in an org
    When they open the org context
    Then they see members, identities/SSO, scopes & precedence, and billing
    This is unbuilt — role-gating not wired.

  # gap: T-2
  NO TEST
  Scenario: a lead sees the clients/engagements/audit surfaces in the org context
    Given a user with lead role in an org
    When they open the org context
    Then they see client engagements, confidentiality controls, and audit trail
    This is unbuilt — role-gating not wired.

  # gap: T-2
  NO TEST
  Scenario: a developer sees read-mostly surfaces in the org context
    Given a user with developer role in an org
    When they open the org context
    Then they can read projects, preview a project's constitution, and view members
    This is unbuilt — role-gating not wired.

  NO TEST
  Scenario: create a Dōjō with a starter constitution
    Given a signed-in user
    When they click "Create Dōjō" and choose a plan
    Then the Dōjō is created with a starter constitution (personal ladder + company rung)
    This is unbuilt — no /org/new flow.

  # gap: F-3
  NO TEST
  Scenario: Dōjō sync works end-to-end
    Given a local sensei with a Dōjō membership
    When it contributes a finding
    Then the finding is accumulated, triaged, approved, distributed, and received downstream
    Per the plan's own record (reorg §12): "Dōjō sync never worked" — this is unbuilt end-to-end.

  # gap: F-3
  NO TEST
  Scenario: a Dōjō-authored rule appears in the member's resolved set
    Given an org that authored a mandatory rule at its scope
    When a member whose repo is bound to that org calls get_rules
    Then the org's rule is in the resolved set
    Today it cannot be pinned — the sync path above never completes, so the shared half silently no-ops.

  NO TEST
  Scenario: the personal landing is work-first for members too
    Given a member of a Dōjō
    When they sign in
    Then they land on the personal view (your work) with My Dōjōs section showing orgs + role
    Currently a member still lands on the old management Overview.