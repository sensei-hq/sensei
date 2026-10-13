Feature: Dōjō Project Planning — gate/improvement split and own-baseline ratchet

  # gap: T-1
  NO TEST
  Scenario: a project connects to Jira read-only and imports the plan
    Given a Jira project and a sensei project
    When the tracker is connected
    Then the plan imports from Jira (phases, features, tasks, AC, dependencies)
    This is unbuilt — no Jira adapter code.

  # gap: T-1
  NO TEST
  Scenario: a project connects to ADO read-only and imports the plan
    Given an ADO project and a sensei project
    When the tracker is connected
    Then the plan imports from ADO
    This is unbuilt — no ADO adapter code.

  NO TEST
  Scenario: the gate/improvement split is enforced
    Given a plan with both gates and improvements configured
    When a run outcome is reported
    Then gates block merge on failure and improvements surface in Observatory only

  NO TEST
  Scenario: the baseline is the project's FTR at plan start
    Given a plan registered on a run
    When the plan is registered
    Then the current FTR is recorded as baselineFtr

  NO TEST
  Scenario: the ratchet advances only on sustained improvement
    Given a run with a baseline FTR
    When FTR improves and holds over multiple runs
    Then the baseline advances
    When FTR spikes once then drops
    Then the baseline does not advance

  NO TEST
  Scenario: gating target 1 — FTR ≥ baseline + δ blocks merge
    Given a run whose FTR is below baseline + δ
    When the outcome is reported
    Then the gate fails and merge is blocked

  NO TEST
  Scenario: gating target 2 — no new hard blocks blocks merge
    Given a run that introduces a new hard block
    When the outcome is reported
    Then the gate fails and merge is blocked

  NO TEST
  Scenario: gating target 3 — no new semantic review flags blocks merge
    Given a run that introduces a new semantic review flag
    When the outcome is reported
    Then the gate fails and merge is blocked
    This needs a scoped read (currently ◐).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: gating target 4 — coupling ≤ baseline blocks merge
    Given a run whose coupling exceeds baseline
    When the outcome is reported
    Then the gate fails and merge is blocked
    This is blocked on architecture-metrics family (#156).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: gating target 5 — cohesion ≥ baseline blocks merge
    Given a run whose cohesion falls below baseline
    When the outcome is reported
    Then the gate fails and merge is blocked
    This is blocked on architecture-metrics family (#156).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: gating target 6 — cycles = 0 blocks merge
    Given a run that introduces a new cycle
    When the outcome is reported
    Then the gate fails and merge is blocked
    This is blocked on architecture-metrics family (#156).

  NO TEST
  Scenario: gating target 7 — doc drift ≤ baseline is advisory
    Given a run whose doc drift exceeds baseline
    When the outcome is reported
    Then it surfaces as an improvement in Observatory, does not block merge
    This needs a scoped read (currently ◐).

  NO TEST
  Scenario: client work uses the client's tracker and gates
    Given a project bound to a client Dōjō
    When the plan is created
    Then it uses the client's tracker and the client's gates
    The employer's mandatory rules remain a floor.

  NO TEST
  Scenario: the project-planning console shows tracker sync, gate config, baseline, and ratchet history
    Given a maintainer/admin in the org context
    When they open /org/[slug]/projects/[id]/planning
    Then they see tracker sync status, gate config, baseline FTR, and ratchet history
    This is unbuilt — org context not wired (T-2).