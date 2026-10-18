Feature: Dōjō Project Planning — gate/improvement split, targets, and the ladder

  # gap: T-1
  NO TEST
  Scenario: a project connects to Jira read-only and imports the plan
    Given a Jira project and a sensei project
    When the tracker is connected
    Then the plan imports from Jira (phases, features, tasks, AC, dependencies)
    And nothing is written back to Jira
    This is unbuilt — no Jira adapter code.

  # gap: T-1
  NO TEST
  Scenario: a project connects to ADO read-only and imports the plan
    Given an ADO project and a sensei project
    When the tracker is connected
    Then the plan imports from ADO
    And nothing is written back to ADO
    This is unbuilt — no ADO adapter code.

  NO TEST
  Scenario: the gate/improvement split is enforced
    Given a sprint with both gating targets and tracked insights configured
    When a run outcome is reported
    Then targets block close on failure and insights surface as context only

  NO TEST
  Scenario: an insight never gates
    Given the first-try rate falls below the project's trailing value
    When the sprint scorecard is computed
    Then the first-try rate shows as an insight and blocks nothing

  NO TEST
  Scenario: the baseline is the project's own history, not a borrowed benchmark
    Given a plan registered on a run
    When the plan is registered
    Then each ratcheted reading starts from the project's own trailing baseline

  NO TEST
  Scenario: the ratchet advances only on sustained improvement
    Given a run with a trailing baseline
    When a reading improves and holds over multiple sprints
    Then the baseline advances
    When the reading spikes once then drops
    Then the baseline does not advance

  NO TEST
  Scenario: gating target 1 — hard-rule violations = 0 blocks close
    Given a run that introduces a hard-rule violation
    When the outcome is reported
    Then the target fails and the sprint cannot close without a note

  # gap: coverage-scoped-read
  NO TEST
  Scenario: gating target 2 — coverage ≥ 80% on guarded paths blocks close
    Given a run whose coverage on guarded paths falls below the project value
    When the outcome is reported
    Then the target fails and the sprint cannot close without a note
    This needs a path-scoped coverage read (currently ◐).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: gating target 3 — new layer violations = 0 blocks close
    Given a run that introduces a layer violation
    When the outcome is reported
    Then the target fails and the sprint cannot close without a note
    This is blocked on the architecture-metrics family (#156).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: gating target 4 — new dependency cycles = 0 blocks close
    Given a run that introduces a dependency cycle
    When the outcome is reported
    Then the target fails and the sprint cannot close without a note
    This is blocked on the architecture-metrics family (#156).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: gating target 5 — modules in the zone of pain ≤ 1 only warns
    Given a run that pushes a module into the zone of pain past the project value
    When the outcome is reported
    Then the target warns and the sprint can still close

  # gap: complexity-scoped-read
  NO TEST
  Scenario: gating target 6 — changed functions over complexity 10 ≤ 2 blocks close
    Given a run whose changed functions exceed the project complexity value
    When the outcome is reported
    Then the target fails and the sprint cannot close without a note
    This needs a per-function, changed-function read (currently ◐).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: gating target 7 — new high-severity anti-patterns = 0 only warns
    Given a run that introduces a high-severity anti-pattern
    When the outcome is reported
    Then the target warns and the sprint can still close

  NO TEST
  Scenario: targets inherit most-specific-wins
    Given a target set at the Dōjō and tightened at the project
    When a sprint adds an override for a task type
    Then the task-type value applies to matching tasks
    And every other task inherits the project value
    And a scope with no value inherits its nearest ancestor

  NO TEST
  Scenario: a target can be turned off
    Given a target whose mode is off
    When a run outcome is reported
    Then the target is not evaluated and blocks nothing

  NO TEST
  Scenario: a sprint that misses a blocking target cannot close without a note
    Given a sprint that misses a blocking target
    When the maintainer closes the sprint
    Then a note is required and the close is recorded with it

  NO TEST
  Scenario: an unlinked session counts toward no task
    Given a session whose branch, commit and prompt carry no ticket key
    When the sprint linkage is computed
    Then the session is listed as unlinked and counts toward no task

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
    Then they see tracker sync status, gate config, the baseline, and the ratchet history
    This is unbuilt — org context not wired (T-2).
MD
echo "feature 1: $(wc -l < docs/prd/modules/04-team/03-dojo-project-planning/scenarios/dojo-project-planning.feature) lines"