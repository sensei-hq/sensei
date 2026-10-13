Feature: Dōjō Project Planning — the unbuilt tracker adapters and scoped reads

  # gap: T-1
  NO TEST
  Scenario: the Jira adapter reads projects, issues, and sprints
    Given a Jira instance with projects
    When the adapter syncs
    Then it reads projects, issues, sprints, and maps to PlanGraph
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: the Jira adapter exports run status back to issues
    Given a run with task updates
    When the adapter exports
    Then issue statuses are updated in Jira
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: the ADO adapter reads projects, work items, and iterations
    Given an ADO instance with projects
    When the adapter syncs
    Then it reads projects, work items, iterations, and maps to PlanGraph
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: the ADO adapter exports run status back to work items
    Given a run with task updates
    When the adapter exports
    Then work item statuses are updated in ADO
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: tracker adapters are read-only — they never write back without explicit export
    Given a tracker adapter
    When it syncs
    Then it only reads; export is a separate explicit action
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: gating target 3 — semantic review flags need a scoped read
    Given a project with semantic review enabled
    When a run outcome is reported
    Then the gate reads only the project's semantic review flags
    This needs a scoped read (currently ◐).

  # gap: T-1
  NO TEST
  Scenario: gating target 7 — doc drift needs a scoped read
    Given a project with doc drift tracking
    When a run outcome is reported
    Then the improvement reads only the project's doc drift
    This needs a scoped read (currently ◐).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: architecture metric: coupling is computed and rated
    Given a project's code graph
    When the architecture metrics run
    Then coupling is computed, rated, and eligible for gating
    This is blocked on #156.

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: architecture metric: cohesion is computed and rated
    Given a project's code graph
    When the architecture metrics run
    Then cohesion is computed, rated, and eligible for gating
    This is blocked on #156.

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: architecture metric: cycles are detected and counted
    Given a project's code graph
    When the architecture metrics run
    Then cycles are detected, counted, and eligible for gating
    This is blocked on #156.