Feature: Harness & CI — the e2e known reds and release automation gaps

  # gap: T-3
  NO TEST
  Scenario: the e2e suite runs and known reds are documented
    Given the Playwright e2e suite
    When it runs in CI
    Then the T-3 known reds are visible in the run summary and do not fail the PR check
    This is a known state (P1), not a build gap.

  NO TEST
  Scenario: the release automation runs the canonical order
    Given a release candidate tag
    When the release workflow runs
    Then it bumps version, publishes crates, updates Homebrew formula, tags git, creates GitHub Release
    This is partially automated; the Homebrew formula PR is manual.

  NO TEST
  Scenario: the Homebrew formula PR is opened automatically
    Given a published crate release
    When the release workflow runs
    Then a PR is opened to homebrew-tap with the new version and sha256
    This is unbuilt — currently manual.

  # gap: D-12
  NO TEST
  Scenario: @rokkit/* and @kavach/* are not held back by lockfiles
    Given the package.json lockfiles
    When dependencies are updated
    Then rokkit and kavach packages are current
    This is a lockfile maintenance gap (P3).

  # gap: D-13
  NO TEST
  Scenario: transitive OSV advisories are monitored
    Given the dependency tree
    When OSV is scanned
    Then the 28 medium advisories are tracked and accepted as non-code-fixable
    This is a known state (P3), not a build gap.