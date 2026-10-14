Feature: Dōjō Project Planning — the unbuilt tracker adapters and scoped reads

  # gap: T-1
  NO TEST
  Scenario: the Jira adapter reads projects, issues, sprints and task types
    Given a Jira instance with projects and a board
    When the adapter syncs
    Then it reads projects, issues, sprints and task types, and maps them to PlanGraph
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: the ADO adapter reads projects, work items and iterations
    Given an ADO instance with projects and an area
    When the adapter syncs
    Then it reads projects, work items and iterations, and maps them to PlanGraph
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: the GitHub Issues adapter reads issues and labels as task types
    Given a GitHub repo with issues and labels
    When the adapter syncs
    Then it reads issues and maps labelled issues to task types
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: tracker adapters are read-only — they never write back
    Given a tracker adapter
    When it syncs
    Then it only reads; there is no export action and no write path
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: exactly one tracker owns sprints
    Given two trackers connected to one project
    When sprints are synced
    Then one tracker owns the sprint calendar and the other contributes tasks only
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: sessions are linked to tasks by key
    Given a project with a key pattern of AUTH-\d+
    When sessions carry the key in their branch, commit or prompt
    Then those sessions count toward the matching task
    And sessions with no key are counted as unlinked and count toward no task
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: Slack notifications fire on misses, closes and long gate waits
    Given Slack notifications are connected
    When a blocking target is missed mid-sprint, a sprint closes, or a session waits on a gate over 30m
    Then the configured hook posts to the channel
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: target 1 reads hard-rule violations from the checkers
    Given a project with adopted checker rules
    When a run outcome is reported
    Then hard-rule violations are read from rule_check_runs and gated
    The checker substrate is built; the gate wiring is not.

  # gap: coverage-scoped-read
  NO TEST
  Scenario: target 2 needs a path-scoped coverage read
    Given a project with lcov coverage
    When a run outcome is reported
    Then coverage is read only for the guarded paths (auth/**, payments/**)
    This needs a path-scoped read (currently ◐).

  # gap: complexity-scoped-read
  NO TEST
  Scenario: target 6 needs a per-function, changed-function complexity read
    Given a project with qlty complexity data
    When a run outcome is reported
    Then complexity is read per changed function
    This needs a new read (currently project-level only).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: architecture metric — layer conformance is computed and rated
    Given a project's code graph
    When the architecture metrics run
    Then layer violations are detected, rated, and eligible for gating
    This is blocked on #156.

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: architecture metric — dependency cycles are detected and counted
    Given a project's code graph
    When the architecture metrics run
    Then cycles are detected, counted, and eligible for gating
    This is blocked on #156.

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: architecture metric — the zone of pain is computed
    Given a project's code graph
    When the architecture metrics run
    Then modules are placed by instability × abstractness and the zone of pain is counted
    This is blocked on #156.

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: the anti-pattern detector reports high-severity anti-patterns
    Given a project's code graph
    When the anti-pattern detector runs
    Then new high-severity anti-patterns are reported and eligible for gating
    This is blocked on #156 (patterns/ absent).
