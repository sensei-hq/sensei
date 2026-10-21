Feature: Relay — the drive loop, watchdog, limits, gate and Dōjō protocol

  # pinned: running_run_heartbeats_and_logs
  Scenario: a running run heartbeats and logs progress
    Given a running run with a registered plan
    When the drive loop ticks
    Then it heartbeats and logs progress

  # pinned: paused_run_is_noop
  Scenario: a paused run is a no-op in the drive loop
    Given a paused run
    When the drive loop ticks
    Then it is a no-op

  # pinned: terminal_run_is_noop
  Scenario: a terminal run is a no-op in the drive loop
    Given a done or failed run
    When the drive loop ticks
    Then it is a no-op

  # pinned: stalled_run_is_noop_watchdog_owns_it
  Scenario: a stalled run is a no-op — the watchdog owns it
    Given a stalled run
    When the drive loop ticks
    Then it is a no-op and the watchdog handles it

  # pinned: drive_config_is_truthy_for_common_true_values
  Scenario: the drive config reads truthy for common true values
    Given SENSEI_RUN_DRIVE set to "1", "true", "yes", "on"
    When the config is read
    Then it is truthy

  # pinned: short_label_is_first_line_bounded
  Scenario: the short label is the first line, bounded
    Given a multi-line goal
    When the short label is taken
    Then it is the first line, bounded

  # pinned: outcome_the_exact_five_day_run_string_is_a_limit_pause
  Scenario: the exact "5-day run" string is a limit pause
    Given agent output containing the exact five-day run message
    When the outcome is mapped
    Then it is a limit pause

  # pinned: outcome_limit_supersedes_a_nonzero_exit
  Scenario: a limit supersedes a non-zero exit
    Given an agent output with a limit and a non-zero exit
    When the outcome is mapped
    Then the limit supersedes

  # pinned: outcome_limit_without_parseable_time_uses_default_backoff
  Scenario: a limit without parseable time uses default backoff
    Given a limit message with no parseable reset time
    When the outcome is mapped
    Then the default backoff is used

  # pinned: outcome_clean_exit_no_limit_is_done
  Scenario: a clean exit with no limit is done
    Given an agent output with exit 0 and no limit
    When the outcome is mapped
    Then it is done

  # pinned: outcome_timeout_is_stalled
  Scenario: a timeout is stalled
    Given an agent output that timed out
    When the outcome is mapped
    Then it is stalled

  # pinned: outcome_nonzero_exit_no_limit_is_flagged
  Scenario: a non-zero exit with no limit is flagged
    Given an agent output with non-zero exit and no limit
    When the outcome is mapped
    Then it is flagged

  # pinned: enabled_drive_with_stub_emits_feature_started_then_done
  Scenario: enabled drive with a stub emits feature started then done
    Given SENSEI_RUN_DRIVE=1 and a stub agent
    When the drive runs
    Then it emits feature started then done

  # pinned: enabled_drive_blocked_by_ask_always_author_stance
  Scenario: enabled drive blocked by ask — always author stance
    Given the author stance is enforced
    When the drive asks
    Then it blocks

  # pinned: enabled_drive_without_project_flags_and_does_not_spawn
  Scenario: enabled drive without a workspace flags and does not spawn
    Given a run with no workspace
    When the drive runs
    Then it flags and does not spawn

  # pinned: enabled_drive_with_empty_goal_flags
  Scenario: enabled drive with an empty goal flags
    Given a run with an empty goal
    When the drive runs
    Then it flags

  # pinned: enabled_drive_nonzero_exit_flags_and_stays_running
  Scenario: enabled drive with non-zero exit flags and stays running
    Given a stub agent that exits non-zero
    When the drive runs
    Then it flags and the run stays running

  # pinned: limit_stub_script
  Scenario: the limit stub script emits limit strings
    Given the limit stub script
    When it is run
    Then it emits the limit strings

  # pinned: running_with_fresh_progress_is_healthy
  Scenario: a running run with fresh progress is healthy
    Given a running run with progress within the window
    When the watchdog checks
    Then it is healthy

  # pinned: running_with_stale_progress_stalls
  Scenario: a running run with stale progress stalls
    Given a running run with progress outside the window
    When the watchdog checks
    Then it stalls

  # pinned: running_progress_boundary_is_inclusive_fresh
  Scenario: the progress boundary is inclusive for fresh
    Given progress exactly at the boundary
    When the watchdog checks
    Then it is fresh

  # pinned: stalled_with_fresh_heartbeat_stays_stalled
  Scenario: a stalled run with a fresh heartbeat stays stalled
    Given a stalled run with a fresh heartbeat
    When the watchdog checks
    Then it stays stalled

  # pinned: stalled_and_heartbeat_stale_recovers_while_under_the_cap
  Scenario: a stalled run with a stale heartbeat recovers while under the cap
    Given a stalled run with stale heartbeat and recoveries < 3
    When the watchdog checks
    Then it recovers

  # pinned: stalled_and_heartbeat_stale_crashes_at_and_over_the_cap
  Scenario: a stalled run with a stale heartbeat crashes at/over the cap
    Given a stalled run with stale heartbeat and recoveries >= 3
    When the watchdog checks
    Then it crashes

  # pinned: paused_blocked_terminal_are_healthy_even_when_stale
  Scenario: paused, blocked, terminal runs are healthy even when stale
    Given a paused, blocked, or terminal run with stale timestamps
    When the watchdog checks
    Then it is healthy

  # pinned: future_timestamps_are_healthy
  Scenario: future timestamps are healthy
    Given a run with future timestamps
    When the watchdog checks
    Then it is healthy

  # pinned: detects_the_exact_five_day_run_message
  Scenario: the limit detector catches the exact five-day run message
    Given output containing the exact five-day run string
    When the detector runs
    Then it is detected

  # pinned: detects_24h_clock_variant
  Scenario: the limit detector catches the 24h clock variant
    Given output with a 24h clock time
    When the detector runs
    Then it is detected

  # pinned: detects_weekly_cap_weekday_variant
  Scenario: the limit detector catches the weekly cap weekday variant
    Given output with a weekly cap weekday
    When the detector runs
    Then it is detected

  # pinned: detects_explicit_month_day_variant
  Scenario: the limit detector catches the explicit month-day variant
    Given output with an explicit month-day
    When the detector runs
    Then it is detected

  # pinned: normal_output_is_not_a_limit
  Scenario: normal output is not a limit
    Given normal agent output
    When the detector runs
    Then it is not a limit

  # pinned: limit_without_parseable_time_yields_none_reset
  Scenario: a limit without parseable time yields no reset
    Given a limit message with no parseable time
    When the detector runs
    Then the reset is none

  # pinned: detection_is_case_insensitive
  Scenario: limit detection is case insensitive
    Given a limit message in mixed case
    When the detector runs
    Then it is detected

  # pinned: next_occurrence_rolls_to_tomorrow_when_time_has_passed
  Scenario: the next occurrence rolls to tomorrow when the time has passed
    Given a local time that has already passed today
    When the next occurrence is computed
    Then it is tomorrow

  # pinned: next_occurrence_uses_today_when_time_is_ahead
  Scenario: the next occurrence uses today when the time is ahead
    Given a local time that is still ahead today
    When the next occurrence is computed
    Then it is today

  # pinned: resolve_uses_reset_plus_jitter_when_present
  Scenario: resolve uses the detected reset plus jitter when present
    Given a limit with a detected reset time
    When the next reset is resolved
    Then it uses the reset plus jitter

  # pinned: resolve_uses_default_backoff_when_reset_absent
  Scenario: resolve uses the default backoff when reset is absent
    Given a limit with no detected reset
    When the next reset is resolved
    Then it uses the default backoff

  # pinned: parse_clock_time_handles_am_pm_and_24h
  Scenario: clock time parsing handles am/pm and 24h
    Given clock times in am/pm and 24h formats
    When they are parsed
    Then both are handled

  # pinned: parse_clock_time_handles_bare_hour_am_pm
  Scenario: clock time parsing handles bare hour with am/pm
    Given a bare hour with am/pm
    When it is parsed
    Then it is handled

  # pinned: detects_bare_hour_pm_with_iana_zone
  Scenario: bare hour pm with IANA zone is detected
    Given a bare hour pm with an IANA zone
    When it is parsed
    Then it is detected

  # pinned: detects_bare_hour_am_with_bullet_separator
  Scenario: bare hour am with bullet separator is detected
    Given a bare hour am with a bullet separator
    When it is parsed
    Then it is detected

  # pinned: detects_bare_hour_reset_at_variants
  Scenario: bare hour reset-at variants are detected
    Given bare hour reset-at variants
    When they are parsed
    Then they are detected

  # pinned: hard_block_main_branch_pushes
  Scenario: a main branch push is a hard block
    Given a bash command `git push --force origin main`
    When the gate classifies
    Then it is a hard block

  # pinned: hard_block_history_rewrite
  Scenario: a history rewrite is a hard block
    Given a bash command `git rebase -i HEAD~5`
    When the gate classifies
    Then it is a hard block

  # pinned: hard_block_destructive_deletes
  Scenario: a destructive delete is a hard block
    Given a bash command `rm -rf /`
    When the gate classifies
    Then it is a hard block

  # pinned: hard_block_deploy_publish_bump
  Scenario: a deploy/publish/bump is a hard block
    Given a bash command `npm publish` or `aws deploy` or `cargo publish`
    When the gate classifies
    Then it is a hard block

  # pinned: hard_block_credentials
  Scenario: credential exposure is a hard block
    Given a bash command with `AWS_SECRET_ACCESS_KEY=...`
    When the gate classifies
    Then it is a hard block

  # pinned: hard_block_money_providers
  Scenario: a money provider command is a hard block
    Given a bash command with `stripe` or `paypal` or `braintree`
    When the gate classifies
    Then it is a hard block

  # pinned: allow_normal_progress_commands
  Scenario: normal progress commands are allowed
    Given a bash command like `cargo test` or `npm run build`
    When the gate classifies
    Then it is allowed

  # pinned: rm_rf_safe_build_dirs_are_allowed
  Scenario: rm -rf on safe build dirs is allowed
    Given a bash command `rm -rf target/` or `rm -rf dist/`
    When the gate classifies
    Then it is allowed

  # pinned: hard_block_classifier_gap_fixes_p3_5
  Scenario: the hard block classifier fixes the P3.5 gaps
    Given the P3.5 gap commands
    When the gate classifies
    Then they are blocked

  # pinned: hard_block_security_review_p3_fixes
  Scenario: the hard block security review fixes the P3 gaps
    Given the P3 security review commands
    When the gate classifies
    Then they are blocked

  # pinned: hard_block_write_edit_to_secret_files
  Scenario: a write/edit to a secret file is a hard block
    Given a write/edit to `.env` or `secrets.yaml`
    When the gate classifies
    Then it is a hard block

  # pinned: allow_write_edit_to_normal_source
  Scenario: a write/edit to normal source is allowed
    Given a write/edit to `src/main.rs`
    When the gate classifies
    Then it is allowed

  # pinned: other_tools_and_malformed_input_are_progress
  Scenario: other tools and malformed input are progress
    Given a non-bash tool or malformed input
    When the gate classifies
    Then it is progress

  # pinned: word_match_respects_boundaries
  Scenario: word matching respects boundaries
    Given a needle that is a substring of another word
    When word match runs
    Then it does not match

  # pinned: semantic_review_flags_obfuscation_markers
  Scenario: semantic review flags obfuscation markers
    Given a bash command with `base64 -d` or `eval` or `curl | sh`
    When the semantic review runs
    Then it flags for review

  # pinned: semantic_review_stays_rare_on_benign
  Scenario: semantic review stays rare on benign commands
    Given a benign bash command
    When the semantic review runs
    Then it does not flag

  # pinned: semantic_review_ok_to_overlap_deterministic_block
  Scenario: semantic review may overlap with deterministic block
    Given a command that is both a hard block and semantically suspicious
    When the gate classifies
    Then both apply

  # pinned: semantic_review_only_bash
  Scenario: semantic review only runs on bash
    Given a non-bash tool call
    When the semantic review runs
    Then it does not run

  # pinned: parse_semantic_verdict_dangerous_true_yields_block
  Scenario: a semantic verdict with dangerous=true yields a block
    Given a semantic verdict with dangerous: true
    When it is parsed
    Then it yields a block

  # pinned: parse_semantic_verdict_dangerous_false_is_none
  Scenario: a semantic verdict with dangerous=false is none
    Given a semantic verdict with dangerous: false
    When it is parsed
    Then it is none

  # pinned: parse_semantic_verdict_tolerates_fences_and_prose
  Scenario: semantic verdict parsing tolerates fences and prose
    Given a verdict wrapped in markdown fences or prose
    When it is parsed
    Then it is tolerated

  # pinned: parse_semantic_verdict_unknown_category_maps_to_default
  Scenario: an unknown semantic category maps to default
    Given a verdict with an unknown category
    When it is parsed
    Then it maps to default

  # pinned: parse_semantic_verdict_missing_reason_is_none
  Scenario: a missing reason is none
    Given a verdict with no reason
    When it is parsed
    Then the reason is none

  # pinned: parse_semantic_verdict_garbage_is_none
  Scenario: garbage input is none
    Given garbage input
    When it is parsed
    Then it is none

  # pinned: semantic_prompt_builders_are_structured
  Scenario: semantic prompt builders are structured
    Given the semantic prompt builders
    When they are inspected
    Then they are structured

  # pinned: relay_run_status_db_strings_match_ddl
  Scenario: relay run status DB strings match the DDL enum
    Given the RelayRunStatus enum
    When the DB strings are compared to the DDL
    Then they match

  # pinned: segment_state_db_strings_match_ddl
  Scenario: segment state DB strings match the DDL enum
    Given the SegmentState enum
    When the DB strings are compared to the DDL
    Then they match

  # pinned: gate_severity_db_strings_match_ddl
  Scenario: gate severity DB strings match the DDL enum
    Given the GateSeverity enum
    When the DB strings are compared to the DDL
    Then they match

  # pinned: relay_inbox_kind_db_strings_match_ddl
  Scenario: relay inbox kind DB strings match the DDL enum
    Given the RelayInboxKind enum
    When the DB strings are compared to the DDL
    Then they match

  # pinned: relay_message_direction_db_strings_match_ddl
  Scenario: relay message direction DB strings match the DDL enum
    Given the RelayMessageDirection enum
    When the DB strings are compared to the DDL
    Then they match

  # pinned: relay_inbox_status_db_strings_match_ddl
  Scenario: relay inbox status DB strings match the DDL enum
    Given the RelayInboxStatus enum
    When the DB strings are compared to the DDL
    Then they match

  # pinned: run_status_helpers
  Scenario: run status helpers work correctly
    Given the run status helpers
    When they are tested
    Then they work

  # pinned: session_update_round_trips_and_omits_none
  Scenario: session update round-trips and omits None
    Given a session update with some None fields
    When it round-trips
    Then the None fields are omitted

  # pinned: segment_round_trips_phase_and_gate
  Scenario: a segment round-trips with phase and gate
    Given a segment with phase and gate
    When it round-trips
    Then they are preserved

  # pinned: segment_carries_plan_metadata_and_omits_none
  Scenario: a segment carries plan metadata and omits None
    Given a segment with plan metadata and None fields
    When it round-trips
    Then the metadata is preserved and None fields omitted

  # pinned: inbox_item_round_trips_and_defaults_pending
  Scenario: an inbox item round-trips and defaults to pending
    Given an inbox item
    When it round-trips
    Then it defaults to pending

  # pinned: inbox_pull_round_trips_with_cursor
  Scenario: an inbox pull round-trips with a cursor
    Given an inbox pull with a cursor
    When it round-trips
    Then the cursor is preserved

  # pinned: reply_round_trips
  Scenario: a reply round-trips
    Given a reply
    When it round-trips
    Then it is preserved

  # pinned: inbox_ack_round_trips
  Scenario: an inbox ack round-trips
    Given an inbox ack
    When it round-trips
    Then it is preserved

  # pinned: session_ack_round_trips
  Scenario: a session ack round-trips
    Given a session ack
    When it round-trips
    Then it is preserved

  # pinned: segments_publish_round_trips
  Scenario: segments publish round-trips
    Given segments to publish
    When they round-trip
    Then they are preserved