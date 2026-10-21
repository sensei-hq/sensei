Feature: Planner & Baseline — the runs API and the unbuilt baseline/ratchet

  # pinned: get_run_bad_uuid_is_400
  Scenario: getting a run with a bad UUID is 400
    Given a malformed UUID
    When the run is requested
    Then the response is 400

  # pinned: get_run_unknown_is_404
  Scenario: getting an unknown run is 404
    Given a UUID that does not exist
    When the run is requested
    Then the response is 404

  # pinned: create_run_empty_goal_is_400
  Scenario: creating a run with an empty goal is 400
    Given a run creation with an empty goal
    When it is submitted
    Then the response is 400

  # pinned: create_run_unresolvable_project_is_400
  Scenario: creating a run for an unresolvable workspace is 400
    Given a workspace id that cannot be resolved
    When the run is created
    Then the response is 400

  # gap: C-1
  NO TEST
  Scenario: the planner persists the plan to the database
    Given a generated PlanGraph
    When it is persisted
    Then it is stored with its phases, features, tasks and dependencies
    This is unbuilt — no plan tables in DDL.

  # gap: C-1
  NO TEST
  Scenario: the baseline FTR is recorded at plan start
    Given a run with a registered plan
    When the plan is registered
    Then the current FTR is recorded as baselineFtr
    This is unbuilt — no baseline_ftr column, no plan_baselines table.

  # gap: C-1
  NO TEST
  Scenario: the ratchet advances the baseline only on sustained improvement
    Given a run with a baseline FTR
    When the FTR improves and holds
    Then the baseline advances
    When the FTR spikes then drops
    Then the baseline does not advance
    This is unbuilt — no ratchet code, no definition_of_done.

  # gap: C-1
  NO TEST
  Scenario: each task has a definition of done that must be met
    Given a task with acceptance criteria
    When the task is marked done
    Then the criteria are verified
    This is unbuilt — no definition_of_done.

  # gap: C-1
  NO TEST
  Scenario: the Tasks tab in the workspace window renders the plan
    Given a workspace with a persisted plan
    When the Tasks tab opens
    Then it shows phases, tasks, states and progress
    This is unbuilt — blocked on planner persistence (C-1).

  # gap: D-6
  NO TEST
  Scenario: a Linear tracker adapter syncs task state
    Given a Linear workspace and a sensei task
    When the task state changes
    Then Linear is updated
    This is unbuilt — D-6 tracker adapters (T3).

  # gap: D-6
  NO TEST
  Scenario: a Jira tracker adapter syncs task state
    Given a Jira workspace and a sensei task
    When the task state changes
    Then Jira is updated
    This is unbuilt — D-6 tracker adapters (T3).

  # gap: D-6
  NO TEST
  Scenario: a GitHub Issues tracker adapter syncs task state
    Given a GitHub repo and a sensei task
    When the task state changes
    Then the GitHub issue is updated
    This is unbuilt — D-6 tracker adapters (T3).