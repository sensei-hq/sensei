Feature: Capture & Sessions — the four ingest paths that turn folders and assistant runs into knowledge

  # pinned: scan_root_enqueues_only_git_folders
  Scenario: a scan root enqueues only git folders as scan work
    Given a scan root containing git repos, standalone folders, and libraries
    When the scan root is enqueued
    Then only the git folders are enqueued for scanning

  # pinned: scan_root_skips_excluded_subtree
  Scenario: an excluded subtree is not scanned
    Given an exclusion pattern covering a subtree
    When the scan root is enqueued
    Then the excluded subtree is skipped

  # pinned: scan_root_errors_on_nonexistent_path
  Scenario: a nonexistent scan root is an explicit error, not a silent skip
    Given a path that does not exist
    When it is added as a scan root
    Then the daemon returns an explicit error

  # pinned: a_scanned_repository_records_its_project_in_the_junction
  Scenario: a scanned repository is bound to its project through the junction
    Given a repository under a watched root
    When it finishes scanning
    Then the repository row reaches its project via the folder-project junction

  # pinned: a_second_scan_of_an_unchanged_repo_keeps_the_parse_state
  Scenario: an unchanged repo re-scan keeps its parse state
    Given a repo that was scanned before
    When it is scanned again without changes
    Then the previous parse state is preserved end-to-end

  # pinned: a_batch_resolves_against_the_watchers_own_roots_without_a_database
  Scenario: the watcher resolves a batch against its own registered roots
    Given a watcher with registered roots
    When a batch of filesystem events arrives
    Then the batch resolves against the watcher's own roots without a database round-trip

  # pinned: a_watcher_batch_is_never_exhaustive
  Scenario: a watcher batch is bounded
    Given a burst of filesystem events
    When the watcher collapses the burst into a pass
    Then no batch is treated as exhaustive of the root

  # pinned: hook_event_returns_decodable_json_ack
  Scenario: a hook event persists as a decodable row and acks
    Given a Claude Code hook fires
    When it is ingested at the hooks endpoint
    Then the event is acknowledged with a decodable payload response

  # pinned: enrich_assistant_events_derives_attrs_from_own_payload
  Scenario: assistant events are enriched from their own payload
    Given raw assistant events
    When they are enriched
    Then derived attributes come from the event payload itself

  # pinned: analyze_project_enriches_sessions_and_writes_turns
  Scenario: the analyzer materialises sessions and writes turns
    Given pending assistant events
    When the analyzer tick materialises sessions
    Then sessions are enriched and turns are written

  Scenario: a secret-bearing prompt is redacted at capture
    Given a prompt payload matching the secret pattern list
    When it is captured
    Then the payload is redacted and the redaction is logged
    And nothing egresses to a remote host

  Scenario: a branch switch flips the active view to the target branch
    Given a folder scanned on branch "main"
    When the git HEAD moves to "feat-x"
    Then unchanged rows are re-tagged for feat-x
    And added files are enqueued as new scan work
    And the historical main rows stay tagged main

  # gap: C-11
  Scenario: transcript scanning is scoped and reviewed as a task
    Given the transcript-scanning path
    When its review task exists
    Then it is scoped, estimated, and scheduled

  # gap: C-14
  Scenario: a non-Claude assistant family joins through the adapter seam
    Given a new assistant family
    When an adapter is authored following the trait
    Then events ingest without changes to shared daemon code

  # gap: C-13
  Scenario: Claude "mods" install path and migration are verified
    Given a Claude "mods" enabled machine
    When the capture path is exercised
    Then the mods install path works and migrations from plugin hooks are verified