Feature: Observatory — Memories, Sessions, Collective, Dōjō, Logs and scheduled tasks

  # pinned: list_memories_filters_by_status
  Scenario: the memories list filters by status
    Given memories in several statuses
    When the list is filtered by one status
    Then only that status is returned

  # pinned: set_memory_status_accept_proposal
  Scenario: accepting a proposal sets the memory active
    Given a proposed memory
    When the proposal is accepted
    Then its status becomes active

  # pinned: get_memory_detail_includes_7d_telemetry
  Scenario: memory detail carries its seven-day telemetry
    Given a memory with load and outcome history
    When its detail is read
    Then the seven-day telemetry is included

  # pinned: memory_loaded_last_7d_respects_window
  Scenario: the loaded-last-7d count respects its window
    Given loads inside and outside a seven-day window
    When the count is computed
    Then only the in-window loads count

  # pinned: range_to_days_maps_known_chips
  Scenario: session range chips map to day counts
    Given the 7d, 30d, 90d and all chips
    When a chip is applied
    Then it maps to the matching day count

  # pinned: workflow_state_md_renders_only_set_fields_in_stable_order
  Scenario: the workflow-state digest renders only set fields in a stable order
    Given a session with some workflow fields set
    When its digest renders
    Then only set fields appear, in a stable order

  # pinned: default_is_the_conservative_share_nothing_shape
  Scenario: collective preferences default to sharing nothing
    Given a fresh collective-preference store
    When it is read
    Then it defaults to the conservative share-nothing shape

  # pinned: allowed_sets_match_ddl_and_the_attribution_enum
  Scenario: the allowed sets match the DDL and the attribution enum
    Given the collective-preference vocabularies
    When they are compared to the DDL and the enum
    Then they match

  # pinned: from_row_folds_partial_stored_toggles_over_full_defaults
  Scenario: stored toggles fold over the full defaults
    Given a partially stored collective preference
    When it is read
    Then the stored toggles fold over the full defaults

  # pinned: a_complete_body_is_trimmed_not_merely_accepted
  Scenario: a Dōjō connection body is trimmed, not merely accepted
    Given a connection body with surrounding whitespace
    When it is submitted
    Then each field is trimmed before it is stored

  # pinned: each_blank_field_is_named_in_its_own_refusal
  Scenario: each blank field is named in its own refusal
    Given a connection body with blank fields
    When it is submitted
    Then each blank field is named in the refusal

  # pinned: a_dead_or_rejected_session_is_401_but_a_blip_is_503
  Scenario: a dead session is 401 and a blip is 503
    Given a rejected Dōjō session
    When the daemon calls out
    Then it is reported 401
    Given a transient Dōjō fault
    When the daemon calls out
    Then it is reported 503

  # pinned: parse_since_relative_durations
  Scenario: the logs filter parses relative durations
    Given a relative duration like 2h
    When the log filter parses it
    Then it maps to a time bound

  # pinned: parse_since_rejects_garbage
  Scenario: the logs filter rejects unparseable input
    Given a garbage duration
    When the log filter parses it
    Then it is rejected rather than silently ignored

  # pinned: the_listing_comes_from_the_table_not_a_static_registry
  Scenario: scheduled tasks list from the table, not a static registry
    Given scheduled tasks persisted in the table
    When the task strip is listed
    Then the rows come from the table

  # pinned: patching_a_worker_that_does_not_exist_is_a_404
  Scenario: patching an unknown scheduled worker is a 404
    Given a worker id that does not exist
    When it is patched
    Then the response is 404

  # pinned: create_and_list_solutions
  Scenario: the workspaces index lists what is created
    Given a workspace is created
    When the workspaces index is listed
    Then the created workspace appears

  NO TEST
  Scenario: the Upgrades lane applies, mutes and pins a downstream finding
    Given a downstream finding in the upgrades lane
    When it is applied, muted or pinned
    Then its state changes and unread counts stay consistent
