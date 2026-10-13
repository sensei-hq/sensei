Feature: Analyzer — the incremental scheduler tick and enrichment contract

  # pinned: projects_due_picks_new_then_only_advanced
  Scenario: the scheduler picks only projects with genuinely new sessions
    Given a warm daemon with several indexed projects
    And some projects have sessions newer than the persisted watermark
    And others have none
    When the analyzer tick runs
    Then only the projects with new sessions are due
    And the unchanged projects are not re-analyzed

  # pinned: due_project_enqueues_analyze_and_doc_drift
  Scenario: a due project enqueues its analysis work
    Given a project that is due
    When the tick resolves it
    Then an AnalyzeProject task is enqueued
    And a doc-drift scan is enqueued for it

  # pinned: watermark_survives_serialize_roundtrip
  Scenario: the watermark survives a daemon restart
    Given a persisted per-project watermark
    When the daemon restarts and the watermark is restored
    Then only projects newer than the restored watermark are due

  # pinned: deserialize_watermark_tolerates_garbage
  Scenario: a corrupted watermark entry is dropped, not fatal
    Given a watermark row that fails to parse
    When it is read back at boot
    Then the entry is rejected without failing the daemon

  # pinned: full_refresh_due_when_never_run_or_interval_elapsed
  Scenario: the daily full refresh fires only on an elapsed interval
    Given the last full refresh is older than the interval
    When the tick evaluates
    Then a full refresh is due for every project

  # pinned: global_passes_classify_before_aggregate_tool_insights
  Scenario: global passes classify before aggregate tool insights run
    Given a fresh full-refresh window
    When the global passes are ordered
    Then classification completes before aggregate tool insights

  # pinned: analyze_project_enriches_sessions_and_writes_turns
  Scenario: enrichment writes turns and stamps the session atomically
    Given a session that has not been analyzed
    When analyze_project enriches it
    Then its turns are written
    And it is stamped analyzed with ftr and corrections

  # pinned: clean_session_is_completed_first_try
  Scenario: a clean first-try session resolves to FTR-true
    Given a session whose follow-up contains no correction signal
    When the outcome is derived
    Then ftr is true

  # pinned: correction_prompt_drops_ftr_and_marks_corrected
  Scenario: a correction prompt drops FTR and marks the session corrected
    Given a session with a correction in the follow-up
    When the outcome is derived
    Then ftr is false
    And the session is marked corrected

  # pinned: empty_stream_yields_no_metrics
  Scenario: no new activity yields no tick work
    Given no new sessions since the watermark
    When the tick runs
    Then no metrics or signals are written

  # pinned: derive_signals_flags_churn_corrections_and_principles
  Scenario: signal derivation is scoped to the affected folders
    Given an enriched session touching one folder
    When derive_signals runs
    Then only the affected folder's signals are derived
    And pattern rows do not grow unbounded

  # pinned: no_end_no_signal_is_incomplete_not_abandoned
  Scenario: a session without an end signal is incomplete, not abandoned
    Given a session with no terminal event
    When the abandonment pass runs
    Then it is recorded as incomplete
    And never as abandoned

  # pinned: resumed_session_is_never_abandoned
  Scenario: a resumed session is never abandoned
    Given a session that resumes later
    When the abandonment pass runs
    Then it is not flagged abandoned

  # pinned: idle_gap_splits_segments_and_excludes_idle_from_duration
  Scenario: long idle gaps split segments and are excluded from duration
    Given a transcript with a long idle gap
    When segments are derived
    Then the idle time is excluded from active duration

  # pinned: detect_communities_marks_indexing_folder_indexed
  Scenario: community detection runs on the daily full refresh
    Given an indexing folder that is due
    When the community pass runs
    Then communities are assigned deterministically
    And the folder is marked indexed

  # pinned: recommends_clear_winner
  Scenario: a model-preference insight surfaces a clear winner
    Given two models with enough qualifying sessions
    When model insight is derived
    Then a clear-winner recommendation is emitted for the project

  # gap: M7
  Scenario: the low-signal chip notes thin projects instead of guessing
    Given a project whose sessions are sparse
    When the insight surface renders
    Then a low-signal note is shown with the real session count
    And no fabricated "low FTR" is implied

  # gap: M7
  Scenario: a per-model FTR decision aid is available
    Given per-model session aggregates
    When the user inspects a model
    Then an FTR-with-confidence decision aid is rendered