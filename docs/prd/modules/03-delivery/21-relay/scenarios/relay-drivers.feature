Feature: Relay — the three relay drivers and the unbuilt screens

  # pinned: plan_update_maps_entries_to_segments_with_state
  Scenario: a plan update maps entries to segments with state
    Given an ACP plan update with entries
    When it is mapped
    Then each entry becomes a segment with the correct state

  # pinned: empty_plan_maps_to_empty_outline
  Scenario: an empty plan maps to an empty outline
    Given an empty ACP plan
    When it is mapped
    Then the outline is empty

  # pinned: plan_entry_state_covers_all_statuses
  Scenario: plan entry state covers all ACP statuses
    Given ACP plan entries with all statuses
    When their states are mapped
    Then each maps to the correct SegmentState

  # pinned: plan_projection_carries_only_content_no_meta_no_detail
  Scenario: the plan projection carries only content, no meta, no detail
    Given an ACP plan entry with meta and detail
    When it is projected
    Then only the content is carried

  # pinned: tool_call_update_never_projects_code_or_args
  Scenario: a tool call update never projects code or args
    Given an ACP tool call update
    When it is projected
    Then code and args are not projected

  # pinned: tool_call_update_variant_never_projects
  Scenario: a tool call update variant never projects
    Given an ACP tool call update variant
    When it is projected
    Then the variant is not projected

  # pinned: message_and_thought_chunks_never_project
  Scenario: message and thought chunks never project
    Given ACP message and thought chunks
    When they are projected
    Then they are not projected

  # pinned: id_is_acp_and_capability_is_acp
  Scenario: the ACP driver id is "acp" and capability is ACP
    Given the ACP driver
    When its id and capability are read
    Then id is "acp" and capability is ACP

  # pinned: observe_update_runs_the_pure_mapping
  Scenario: observe update runs the pure mapping
    Given an ACP session update
    When observe update runs
    Then it runs the pure mapping

  # pinned: drive_step_is_unsupported_observe_only
  Scenario: the ACP driver drive step is unsupported — observe only
    Given the ACP driver
    When drive step is called
    Then it returns unsupported (observe only)

  # pinned: id_is_claude_and_capability_is_hooks
  Scenario: the Claude driver id is "claude" and capability is hooks
    Given the Claude driver
    When its id and capability are read
    Then id is "claude" and capability is hooks

  # pinned: drive_step_with_stub_cmd_captures_zero_exit
  Scenario: the Claude driver drive step with stub cmd captures zero exit
    Given the Claude driver with a stub command
    When drive step runs
    Then it captures exit 0

  # pinned: drive_step_with_bogus_cmd_is_spawn_error
  Scenario: the Claude driver drive step with bogus cmd is a spawn error
    Given the Claude driver with a bogus command
    When drive step runs
    Then it is a spawn error

  # pinned: recent_activity_is_running
  Scenario: recent activity maps to running
    Given recent agent activity
    When the fallback driver observes
    Then it is running

  # pinned: stale_activity_is_idle
  Scenario: stale activity maps to idle
    Given stale agent activity
    When the fallback driver observes
    Then it is idle

  # pinned: no_activity_is_idle
  Scenario: no activity maps to idle
    Given no agent activity
    When the fallback driver observes
    Then it is idle

  # pinned: terminal_is_done_regardless_of_recency
  Scenario: terminal state is done regardless of recency
    Given a terminal agent state
    When the fallback driver observes
    Then it is done

  # pinned: exact_boundary_is_running
  Scenario: the exact boundary is running
    Given activity exactly at the boundary
    When the fallback driver observes
    Then it is running

  # pinned: just_past_boundary_is_idle
  Scenario: just past the boundary is idle
    Given activity just past the boundary
    When the fallback driver observes
    Then it is idle

  # pinned: future_timestamp_is_running_not_panic
  Scenario: a future timestamp is running, not a panic
    Given a future timestamp
    When the fallback driver observes
    Then it is running, not a panic

  # pinned: now_exactly_is_running
  Scenario: now exactly is running
    Given activity timestamped now
    When the fallback driver observes
    Then it is running

  # pinned: huge_gap_is_idle
  Scenario: a huge gap is idle
    Given a huge time gap since last activity
    When the fallback driver observes
    Then it is idle

  # pinned: zero_idle_after_only_now_is_running
  Scenario: zero idle after only now is running
    Given only a now timestamp
    When the fallback driver observes
    Then it is running

  # pinned: coarse_maps_to_honest_relay_status
  Scenario: coarse observe maps to an honest relay status
    Given coarse observation data
    When the fallback driver observes
    Then the relay status is honest

  # pinned: idle_relay_status_needs_attention
  Scenario: an idle relay status needs attention
    Given an idle relay status
    When it is evaluated
    Then it needs attention

  # pinned: id_carries_underlying_tool
  Scenario: the fallback driver id carries the underlying tool
    Given the fallback driver wrapping a tool
    When its id is read
    Then it carries the underlying tool

  # pinned: bare_fallback_id_when_no_tool
  Scenario: bare fallback id when no underlying tool
    Given the fallback driver with no tool
    When its id is read
    Then it is bare

  # pinned: drive_step_is_unsupported_observe_coarse
  Scenario: the fallback driver drive step is unsupported — observe coarse
    Given the fallback driver
    When drive step is called
    Then it returns unsupported (observe coarse)

  # pinned: coarse_observe_runs_the_pure_core
  Scenario: coarse observe runs the pure core
    Given the fallback driver
    When coarse observe runs
    Then it runs the pure core

  # pinned: driver_for_returns_claude_backend
  Scenario: the driver factory returns the Claude backend for the claude config
    Given the agent config for claude
    When the driver factory runs
    Then it returns the Claude backend

  # pinned: driver_for_carries_cfg_agent_cmd
  Scenario: the driver factory carries the agent cmd from config
    Given a config with agent_cmd
    When the driver factory runs
    Then the driver carries the cmd

  NO TEST
  Scenario: the mobile relay screen shows live drive progress
    Given a running drive
    When the mobile relay screen opens
    Then it shows heartbeats, progress, gates and nudges in real time
    This is greenfield — unbuilt, no test.

  NO TEST
  Scenario: the desktop relay screen shows the plan and live drive
    Given a run with a registered plan
    When the desktop relay screen opens
    Then it shows the plan graph and live drive progress
    This is greenfield — unbuilt, no test.