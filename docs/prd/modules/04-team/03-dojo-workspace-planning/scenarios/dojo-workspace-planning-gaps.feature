Feature: Dōjō Workspace Planning — the unbuilt tracker adapters, the ticket fetch, and scoped reads

  # gap: C-1
  NO TEST
  Scenario: the Jira adapter's mirror maps to the PlanGraph
    Given a Jira instance with a project and a board
    When the daemon's adapter syncs
    Then issues, sprints and task types are mirrored and mapped to the PlanGraph
    The daemon-side adapter and mirror are built (PR #274, the solo/personal path, D49); the PlanGraph mapping is not.

  # gap: T-1
  NO TEST
  Scenario: the ADO adapter reads workspaces, work items and iterations
    Given an ADO instance with workspaces and an area
    When the adapter syncs
    Then it reads workspaces, work items and iterations, and maps them to PlanGraph
    This is unbuilt — mockup only. It goes on the dōjō path (D49).

  # gap: T-1
  NO TEST
  Scenario: the GitHub Issues adapter reads issues and labels as task types
    Given a GitHub repo with issues and labels
    When the adapter syncs
    Then it reads issues and maps labelled issues to task types
    This is unbuilt — mockup only. It goes on the dōjō path (D49).

  # gap: T-1
  NO TEST
  Scenario: an adapter writes nothing while writeback is off
    Given a tracker on a workspace whose writeback is off (the default, D50)
    When it syncs
    Then it only reads
    Writeback (ac7822d2b) is only on branch feature/tracker_metrics_writeback, not on develop.

  # gap: #310
  NO TEST
  Scenario: an org workspace's tracker is connected in the dōjō
    Given an org workspace
    When an admin connects its tracker
    Then the dōjō holds the credential and the members' daemons receive what it mirrors through sync (D49)
    This is unbuilt — the dōjō has no secret store yet (#310).

  # gap: T-1
  NO TEST
  Scenario: exactly one tracker owns sprints
    Given two trackers connected to one workspace
    When sprints are synced
    Then one tracker owns the sprint calendar and the other contributes tasks only
    This is unbuilt — mockup only.

  NO TEST
  Scenario: sessions are linked to tickets by key, as the fallback
    Given a workspace with a key pattern of AUTH-\d+
    When sessions carry the key in their branch or prompt
    Then those sessions count toward the matching ticket
    And sessions with no fetch and no key are counted as unlinked and count toward no ticket
    Branch and prompt linkage is built (session_task_links); commit-message linkage is reserved, not built.

  # gap: ticket-fetch
  NO TEST
  Scenario: the assistant's ticket fetch marks the start of work
    Given a user who hands the assistant the key AUTH-412
    When the assistant fetches the ticket through the daemon's MCP tool
    Then it receives the ticket's title, description, acceptance, status, sprint and assignee
    And with it the ticket's rules and its resolved targets (D62)
    And the session is linked to AUTH-412 and the start of work is recorded (D63)
    This is unbuilt — designed in M14.

  # gap: ticket-fetch
  NO TEST
  Scenario: the sync lists the tickets assigned to the user
    Given a synced tracker with tickets assigned to the signed-in user
    When the user opens their tickets
    Then those tickets are listed with their keys, ready to hand to the assistant (D63)
    This is unbuilt — the mirror stores no assignee today.

  # gap: T-1
  NO TEST
  Scenario: Slack notifications fire on misses, closes and long relay approval waits
    Given Slack notifications are connected
    When a blocking target is missed and not cleared, a sprint closes, or a session waits on a relay approval over 30m
    Then the configured hook posts to the channel
    This is unbuilt — mockup only.

  # gap: T-1
  NO TEST
  Scenario: target 1 reads hard-rule violations from the checkers
    Given a workspace with adopted checker rules
    When a run outcome is reported
    Then hard-rule violations are read from rule_check_runs and evaluated as a target
    The checker substrate is built; the target wiring is not.

  # gap: coverage-scoped-read
  NO TEST
  Scenario: target 2 needs a path-scoped coverage read
    Given a workspace with lcov coverage
    When a run outcome is reported
    Then coverage is read only for the guarded paths (auth/**, payments/**)
    This needs a path-scoped read (currently ◐).

  # gap: complexity-scoped-read
  NO TEST
  Scenario: target 6 needs a per-function, changed-function complexity read
    Given a workspace with qlty complexity data
    When a run outcome is reported
    Then complexity is read per changed function
    This needs a new read (currently workspace-level only).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: architecture metric — layer conformance is computed and rated
    Given a workspace's code graph
    When the architecture metrics run
    Then layer violations are detected, rated, and eligible as a target
    This is blocked on #156.

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: architecture metric — dependency cycles are detected and counted
    Given a workspace's code graph
    When the architecture metrics run
    Then cycles are detected, counted, and eligible as a target
    This is blocked on #156.

  # gap: zone-of-pain-sprint-read
  NO TEST
  Scenario: the zone of pain is counted per sprint
    Given a workspace's code graph
    When a sprint's targets are evaluated
    Then the modules in the zone of pain are counted against the workspace's own baseline
    Placement by instability × abstractness is built (sensei.component_zones, #223); the per-sprint read is not.

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: the anti-pattern detector reports high-severity anti-patterns
    Given a workspace's code graph
    When the anti-pattern detector runs
    Then new high-severity anti-patterns are reported and eligible as a target
    This is blocked on #156 (patterns/ absent).
