Feature: Dōjō Workspace Planning — the target/insight split, targets, the ladder, and the missed-target handoff

  # gap: C-1
  NO TEST
  Scenario: a workspace connects to Jira and imports the plan, writing nothing back by default
    Given a Jira project and a sensei workspace whose writeback is off (the default, D50)
    When the tracker is connected and synced
    Then the plan imports from Jira (phases, features, tasks, AC, dependencies)
    And nothing is written back to Jira
    The daemon-side Jira adapter and sync are built (PR #274); the PlanGraph import is not (C-1).

  # gap: T-1
  NO TEST
  # Usage and SDLC metrics write-back is the one write: spec/daemon/tracker-metrics-writeback.md
  Scenario: a workspace connects to ADO and imports the plan
    Given an ADO workspace and a sensei workspace
    When the tracker is connected
    Then the plan imports from ADO
    And nothing is written back to ADO
    This is unbuilt — no ADO adapter code. New adapters go on the dōjō path (D49).

  NO TEST
  Scenario: the target/insight split holds
    Given a sprint with both targets and tracked insights configured
    When a run outcome is reported
    Then each missed target is reported with what to fix, and insights surface as context only

  NO TEST
  Scenario: an insight is never a target
    Given the first-try rate falls below the workspace's trailing value
    When the sprint scorecard is computed
    Then the first-try rate shows as an insight and blocks nothing

  NO TEST
  Scenario: the baseline is the workspace's own history, not a borrowed benchmark
    Given a plan registered on a run
    When the plan is registered
    Then each ratcheted reading starts from the workspace's own trailing baseline

  NO TEST
  Scenario: the ratchet advances only on sustained improvement
    Given a run with a trailing baseline
    When the trailing 3-sprint median of a reading beats the baseline
    Then the baseline advances
    When the reading spikes once then drops
    Then the baseline does not advance

  NO TEST
  Scenario: target 1 — hard-rule violations = 0 is blocking
    Given a run that introduces a hard-rule violation
    When the outcome is reported
    Then the target reads "target missed (blocking)" and the miss is handed to the assistant with what to fix

  # gap: coverage-scoped-read
  NO TEST
  Scenario: target 2 — coverage ≥ 80% on guarded paths is blocking
    Given a run whose coverage on guarded paths falls below the workspace value
    When the outcome is reported
    Then the target reads "target missed (blocking)" and the miss is handed to the assistant with what to fix
    This needs a path-scoped coverage read (currently ◐).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: target 3 — new layer violations = 0 is blocking
    Given a run that introduces a layer violation
    When the outcome is reported
    Then the target reads "target missed (blocking)" and the miss is handed to the assistant with what to fix
    This is blocked on the architecture-metrics family (#156).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: target 4 — new dependency cycles = 0 is blocking
    Given a run that introduces a dependency cycle
    When the outcome is reported
    Then the target reads "target missed (blocking)" and the miss is handed to the assistant with what to fix
    This is blocked on the architecture-metrics family (#156).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: target 5 — modules in the zone of pain ≤ 1 only warns
    Given a run that pushes a module into the zone of pain past the workspace value
    When the outcome is reported
    Then the target warns and nothing waits for acceptance

  # gap: complexity-scoped-read
  NO TEST
  Scenario: target 6 — changed functions over complexity 10 ≤ 2 is blocking
    Given a run whose changed functions exceed the workspace complexity value
    When the outcome is reported
    Then the target reads "target missed (blocking)" and the miss is handed to the assistant with what to fix
    This needs a per-function, changed-function read (currently ◐).

  # gap: architecture-metrics (#156)
  NO TEST
  Scenario: target 7 — new high-severity anti-patterns = 0 only warns
    Given a run that introduces a high-severity anti-pattern
    When the outcome is reported
    Then the target warns and nothing waits for acceptance

  NO TEST
  Scenario: targets inherit most-specific-wins
    Given a target set at the Dōjō, tightened at the workspace and again for one repository
    When a sprint adds an override for a task type
    Then the task-type value applies to matching tasks in that sprint
    And every other task in that repository inherits the repository value
    And a rung with no value inherits its nearest broader rung (Dōjō → Workspace → Repository → Sprint → Task type → Task, D48)

  NO TEST
  Scenario: a team owns no targets
    Given a team linked to a workspace
    When targets are resolved for a ticket the team works on
    Then no rung belongs to the team, and the team's link only splits the results (D13)

  NO TEST
  Scenario Outline: only the allowed role sets each rung
    Given a person whose role is <role>
    When they set a target at the <rung> rung
    Then the change is <outcome> (D54)

    Examples:
      | role                                          | rung       | outcome  |
      | admin                                         | Workspace  | saved    |
      | maintainer on a repository in the workspace   | Workspace  | refused  |
      | maintainer on that repository                 | Repository | saved    |
      | maintainer on another repository              | Repository | refused  |
      | maintainer on any repository in the workspace | Sprint     | saved    |
      | lead who is not a maintainer                  | Task type  | refused  |
      | viewer                                        | Task       | refused  |

  NO TEST
  Scenario: a target can be turned off
    Given a target whose mode is off
    When a run outcome is reported
    Then the target is not evaluated and blocks nothing

  NO TEST
  Scenario: a missed blocking target is handed to the assistant, not a hard stop
    Given a change that misses a blocking target
    When the targets are evaluated
    Then the assistant receives the missed target and what to fix over MCP (D61, D62)
    And no merge, sprint or tracker state is locked

  NO TEST
  Scenario: a lead or admin accepts a standing blocking miss with a note
    Given a blocking miss the assistant did not clear
    When the dōjō Home is opened by a lead or admin
    Then the miss is in the "needs you" queue (D69)
    And accepting it requires a note, recorded with who accepted it
    And a maintainer who is not a lead or admin cannot accept it (D61)

  NO TEST
  Scenario Outline: the follow-up ticket follows the workspace's writeback setting
    Given an accepted deviation and a workspace whose writeback is <writeback>
    When the lead opens a follow-up ticket
    Then <result> (D50, D61)

    Examples:
      | writeback | result                                                     |
      | on        | the ticket is created in the tracker                       |
      | off       | Sensei drafts the ticket and the lead files it themselves  |

  NO TEST
  Scenario Outline: targets are free except on an org's private repositories
    Given a <dojo> dōjō and a <visibility> repository with no subscription
    When the Targets tab is opened
    Then targets, blocking included, are <state> (D52)

    Examples:
      | dojo     | visibility | state                                                            |
      | personal | private    | available                                                        |
      | org      | public     | available                                                        |
      | org      | private    | shown, saying they are on paid plans, naming the plan and linking to Plan & billing |

  NO TEST
  Scenario: a viewer reads aggregate target results only
    Given a viewer granted one workspace
    When the viewer opens it
    Then metrics come first and target results show in aggregate, never per person (D68, D8)
    And an aggregate over fewer than 2 contributors is withheld (F4)

  NO TEST
  Scenario: an unlinked session counts toward no task
    Given a session whose branch, commit and prompt carry no ticket key
    When the sprint linkage is computed
    Then the session is listed as unlinked and counts toward no task

  NO TEST
  Scenario: a session is governed by its repository's dōjō alone
    Given a person who is a member of two dōjōs
    When a session runs in a repository of the second dōjō
    Then the rules and targets that apply are that dōjō's, plus the person's own user-rung rules (D53)

  NO TEST
  Scenario: the workspace page shows tracker sync, targets, baseline, and ratchet history
    Given a maintainer or admin in the org context
    When they open /org/[slug]/workspaces/[id]
    Then they see tracker sync status, the targets by rung, the baseline, and the ratchet history
    This is unbuilt — the workspace page is built (per-repository metrics), its planning tabs are not.

  NO TEST
  Scenario: a reading from a partial index is not evaluated
    Given a repository still indexing
    When a sprint closes
    Then its index-derived targets read "not measured" and are neither met nor missed

  NO TEST
  Scenario: sprint targets read the development branch
    Given a workspace whose repositories map development to develop and production to main
    When a sprint target is evaluated
    Then it reads develop's latest graph
