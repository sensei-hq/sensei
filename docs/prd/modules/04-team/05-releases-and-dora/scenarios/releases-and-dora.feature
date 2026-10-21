Feature: Releases, DORA & release gates — deployments read from CI, the four keys, gated releases

  # gap: D-1
  NO TEST
  Scenario: a project reads its deployments from GitHub Actions, read-only
    Given a project with a GitHub Actions connection
    When the dōjō syncs the connection
    Then each production deployment is recorded with its commit, environment and outcome
    And nothing is written to GitHub
    This is unbuilt — no pipeline adapter.

  # gap: D-1
  NO TEST
  Scenario: a project reads its deployments from Azure Pipelines, read-only
    Given a project with an Azure Pipelines connection
    When the dōjō syncs the connection
    Then each production deployment is recorded
    And nothing is written to Azure DevOps
    This is unbuilt — no pipeline adapter.

  # gap: D-1
  NO TEST
  Scenario: a project with only release tags lists releases from the tags
    Given a project with no pipeline connection and release tags in git
    When the Releases tab opens
    Then the releases come from the tags
    And the four keys say they are tag-derived
    This is unbuilt.

  # gap: D-1
  NO TEST
  Scenario: a project with no connection and no tags is not measured
    Given a project with no pipeline connection and no release tags
    When the Releases tab opens
    Then it says "not measured"
    And it never shows 0 deployments or a passed gate
    This is unbuilt.

  # gap: D-3
  NO TEST
  Scenario: the four keys match a hand count
    Given a project's production deployments and production incidents over 30 days
    When the Delivery card is read
    Then deployment frequency, lead time, change failure rate and time to restore match a hand count over the same window
    And no key is shown per author
    This is unbuilt.

  # gap: D-2
  NO TEST
  Scenario: a production incident moves change failure rate and time to restore
    Given a production deployment
    When a production incident linked to it is opened and later restored
    Then change failure rate counts that deployment as failed
    And time to restore includes the incident's open-to-restore time
    And a confidentiality incident never counts toward either
    This is unbuilt — dojo.incidents holds confidentiality incidents only.

  # gap: D-4
  NO TEST
  Scenario: a release that adds a hard-rule violation is blocked and says why
    Given a release candidate whose change set adds a hard-rule violation
    When its release gate is evaluated
    Then the release is blocked and names the target
    And a maintainer can mark it shippable only with a note, which is kept
    This is unbuilt.

  # gap: D-4
  NO TEST
  Scenario: the gate reads the release's change set, not the whole project
    Given a project with existing violations from before the last release
    And a release candidate that adds none
    When its release gate is evaluated
    Then the code targets pass
    This is unbuilt.

  # gap: D-4
  NO TEST
  Scenario: an open blocking production incident holds a release
    Given an open blocking production incident on the project
    When a release gate is evaluated
    Then release readiness fails and names the incident
    This is unbuilt.

  # gap: D-4
  NO TEST
  Scenario: the stability guardrail warns by default and blocks only when opted in
    Given a project whose trailing change failure rate is over its threshold
    When a release gate is evaluated
    Then the gate warns and does not block
    And with the guardrail set to block, the release is blocked
    This is unbuilt.

  # gap: B-1
  NO TEST
  Scenario: sensei shows the dōjō's release and gate state
    Given a project whose latest release was blocked in the dōjō
    When the project window opens in sensei
    Then it shows the release, its blocked gate and the failing target
    This is unbuilt.
