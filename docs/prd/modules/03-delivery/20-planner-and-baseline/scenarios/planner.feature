Feature: Planner & Baseline — the v0 planner generate, parse, render and the graph model

  # pinned: sample_json
  Scenario: the planner's sample JSON is valid
    Given the sample JSON string
    When it is parsed
    Then it deserialises to a valid PlanGraph

  # pinned: prompt_carries_goal_and_depth_bar
  Scenario: the planner prompt carries the goal and depth bar
    Given a goal and a depth bar
    When the prompt is built
    Then it includes both

  # pinned: parse_extracts_json_from_prose
  Scenario: the parser extracts JSON from prose
    Given an LLM response with JSON embedded in prose
    When it is parsed
    Then the JSON is extracted

  # pinned: parse_rejects_feature_without_acceptance_criteria
  Scenario: a feature without acceptance criteria is rejected
    Given a PlanGraph with a feature that has no acceptance criteria
    When it is parsed
    Then it is rejected

  # pinned: parse_rejects_empty_goal_or_no_phases
  Scenario: an empty goal or no phases is rejected
    Given a PlanGraph with an empty goal or no phases
    When it is parsed
    Then it is rejected

  # pinned: parse_drops_empty_features_then_rejects_empty_phase
  Scenario: empty features are dropped, then empty phases are rejected
    Given a PlanGraph with empty features then an empty phase
    When it is parsed
    Then empty features are dropped and the empty phase is rejected

  # pinned: render_includes_goal_phases_features_and_criteria
  Scenario: the renderer includes goal, phases, features and criteria
    Given a valid PlanGraph
    When it is rendered to JSON
    Then the output includes the goal, phases, features and acceptance criteria

  # pinned: default_pending
  Scenario: a segment state defaults to pending
    Given a PlanGraph with no explicit state
    When it is deserialised
    Then the default state is pending

  # pinned: phase_rollup
  Scenario: phase state rolls up from tasks
    Given a phase with tasks in various states
    When the rollup runs
    Then the phase state reflects the task states

  # pinned: deserializes_with_pending_default_and_counts
  Scenario: deserialisation keeps pending default and counts
    Given a JSON PlanGraph with pending default
    When it is deserialised
    Then the default and counts are preserved

  # pinned: projects_phase_then_tasks_with_parent_seq_and_labels
  Scenario: the projection orders phases then tasks by parent_seq and labels
    Given a PlanGraph with phases, tasks and parent_seq
    When it is projected
    Then the order is phase-then-tasks by parent_seq and labels

  # pinned: phase_state_rolls_up_from_tasks
  Scenario: phase state rolls up from its tasks
    Given a phase whose tasks include failed and blocked
    When the rollup runs
    Then the phase state is failed or blocked

  # pinned: failure_and_block_dominate_the_rollup
  Scenario: failure and block dominate the rollup
    Given tasks with mixed states including failed and blocked
    When the rollup runs
    Then failed dominates, then blocked

  # pinned: set_task_state_finds_and_updates_or_reports_missing
  Scenario: setting a task state finds it or reports missing
    Given a PlanGraph and a task id
    When the state is set
    Then it updates the task or reports not found

  # pinned: task_progress_counts_terminal_over_total
  Scenario: task progress is terminal over total
    Given a phase with 7 tasks, 3 terminal
    When progress is computed
    Then it is 3/7

  # pinned: round_trips_through_json_with_state
  Scenario: the PlanGraph round-trips through JSON with state
    Given a PlanGraph with task states
    When it is serialised and deserialised
    Then the states are preserved

  # pinned: validate_accepts_a_dag
  Scenario: a valid DAG is accepted
    Given a PlanGraph with a valid DAG
    When it is validated
    Then it is accepted

  # pinned: validate_rejects_duplicate_ids
  Scenario: duplicate ids are rejected
    Given a PlanGraph with duplicate task ids
    When it is validated
    Then it is rejected

  # pinned: validate_rejects_dangling_and_self_deps
  Scenario: dangling and self dependencies are rejected
    Given a PlanGraph with a dangling dep or a self dep
    When it is validated
    Then it is rejected

  # pinned: validate_rejects_a_cycle
  Scenario: a cycle is rejected
    Given a PlanGraph with a cycle
    When it is validated
    Then it is rejected

  # pinned: create_run_happy_path_returns_201_and_running_run
  Scenario: creating a run returns 201 with a running run
    Given a valid goal and project
    When a run is created
    Then it returns 201 and a running run

  # pinned: list_and_get_return_run_with_events
  Scenario: listing and getting a run returns it with events
    Given a created run
    When it is listed or got
    Then it returns with its events

  # pinned: register_plan_rejects_bad_input
  Scenario: registering a bad plan rejects
    Given a run and an invalid PlanGraph
    When the plan is registered
    Then it is rejected

  # pinned: register_plan_stores_graph_and_returns_running_run
  Scenario: registering a valid plan stores the graph and returns the run
    Given a run and a valid PlanGraph
    When the plan is registered
    Then the graph is stored and the run is returned running

  # pinned: update_task_status_flips_state_bad_state_and_unknown_task
  Scenario: updating a task status flips state or rejects bad/unknown
    Given a run with a registered plan
    When a task status is updated
    Then the state flips or rejects bad state / unknown task

  # pinned: update_task_status_404_when_run_has_no_graph
  Scenario: updating a task on a run with no graph is 404
    Given a run with no registered plan
    When a task status is updated
    Then it is 404

  # pinned: report_run_outcome_marks_terminal_or_rejects
  Scenario: reporting a run outcome marks terminal or rejects
    Given a running run
    When an outcome is reported
    Then it marks terminal or rejects

  # pinned: get_pending_nudges_validates_and_fails_soft_without_dojo
  Scenario: getting pending nudges validates and fails soft without Dōjō
    Given a run with pending nudges
    When nudges are requested without Dōjō
    Then it validates and fails soft

  NO TEST
  Scenario: the planner generates a plan graph for a realistic goal
    Given a realistic goal and project context
    When the planner generates
    Then it returns a valid PlanGraph