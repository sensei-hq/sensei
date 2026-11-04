Feature: Website — the unbuilt claim audit and dependency drift

  # gap: D-11
  NO TEST
  Scenario: the claim audit CI fails on untraced claims
    Given the website source
    When the claim audit runs in CI
    Then any feature claim without a roadmap trace fails the build
    This is unbuilt — no claim audit CI.

  # gap: D-11
  NO TEST
  Scenario: a dropped claim (D-11) cannot reappear
    Given the claim audit
    When a previously dropped claim is added
    Then the audit fails the build
    This is unbuilt.

  # resolved: D-12
  NO TEST
  Scenario: @rokkit/* and @kavach/* are not held back by lockfiles
    Given the website package.json
    When dependencies are updated
    Then rokkit and kavach packages are current
    Closed 2026-11-03 by eefc6c119 — every @kavach/* at 1.3.2, one @rokkit
    family at 1.14.0 (#191).

  NO TEST
  Scenario: the claim audit runs on every deploy
    Given a deploy pipeline
    When the site deploys
    Then the claim audit runs and must pass
    This is unbuilt.