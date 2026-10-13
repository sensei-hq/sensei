Feature: Website — the built GitHub Sponsors link

  NO TEST
  Scenario: the GitHub Sponsors link is present and functional
    Given the marketing site
    When the Sponsors link is clicked
    Then it navigates to github.com/sponsors/sensei-hq

  NO TEST
  Scenario: the install command is the unified one-liner
    Given the landing page
    When the install command is displayed
    Then it is `curl -fsSL https://sensei.dev/install.sh | sh`

  NO TEST
  Scenario: the pricing page matches the final tier model
    Given the pricing page
    When it renders
    Then it shows Free (individuals/OSS), Paid (per active contributor), Sponsors
    And it does not show an Enterprise tier

  NO TEST
  Scenario: the changelog is generated from git tags
    Given the changelog page
    When it renders
    Then entries come from git tags, not hand-written content

  NO TEST
  Scenario: every feature claim has a roadmap trace
    Given a feature claim on the site
    When the claim audit runs
    Then it maps to a roadmap status (Built / Partial / Mock-only / Designed / Dormant / Gap)