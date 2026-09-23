---
name: Scenarios
description: Gherkin scenarios for every flow — lifecycle, configuration, events, safety, classification
date: 2026-09-23
status: current
---

# Stage 14 — scenarios

Executable descriptions of every flow. `15-triggers.md` lists what starts them;
`README.md` describes the pipeline they run through.

Each scenario names the test that pins it where one exists, and says **NO TEST**
where none does — an unpinned scenario is a wish, and marking it is what stops
it reading as a guarantee.

---

## Daemon lifecycle

```gherkin
Feature: The index converges regardless of how the daemon stopped

  Scenario: Daemon starts
    Given one or more watch roots are registered
    When the daemon boots
    Then reconcile_scheduler runs its boot tick UNCONDITIONALLY
    And one ScanRoot is enqueued per watch root with scope Full
    And the watcher is registered for each root with that root's exclusions
    # pinned: reconcile_scheduler::tests::enqueue_reconcile_scans_enqueues_one_scanroot_per_root

  Scenario: Daemon restarts after a crash mid-scan
    Given folders were left in a non-terminal state (discovered/queued/indexing/failed)
    When the daemon boots
    Then resume_pending_scans re-enqueues ProcessGitFolder for each such folder
    And the scope is Full, because a crashed scan proved nothing about what exists
    And enqueue_unique makes a re-enqueue of an already-running folder a no-op

  Scenario: Files changed while the daemon was down
    Given files were edited, added and deleted with the daemon stopped
    When the daemon starts
    Then NO filesystem events are delivered for that period
    # notify hardcodes kFSEventStreamEventIdSinceNow; inotify has no history at all.
    And the boot ScanRoot discovers them by DIFF instead
    And absence from the walk IS a valid deletion, because the scope is Full

  Scenario: The daemon binary was upgraded
    Given the running binary's version differs from the last recorded one
    When the daemon boots
    Then version_rescan enqueues ScanRoot per root with scope Full
    # A new binary parses differently; a graph an older one built misrepresents the code.

  Scenario: The daemon never stops
    When 300 seconds elapse with no filesystem events
    Then the reconcile schedule enqueues ScanRoot per root as a safety net
    And a no-op re-scan is stat-only, because the mtime gate reads no file
```

## Root configuration

```gherkin
Feature: Watch roots are configured through the API

  Scenario: A root is added
    When POST /api/workspace/roots succeeds
    Then the root is written to folders_to_watch with its exclusions
    And the live watcher is registered with the RESOLVED absolute prefixes
    And NO scan is enqueued by that call
    # The scan arrives as a separate POST /api/scan from the client.

  Scenario: A root is added inside an existing watch root
    Given ~/Work is already a watch root
    When a scan targets ~/Work/sub
    Then enclosing_watch_root resolves ~/Work
    And no second root row is created
    # pinned: scan::tests::scan_under_existing_root_reuses_it_not_a_new_root

  Scenario: An exclusion is added
    When the excluded list gains an entry
    Then prune_under_prefix deletes that subtree's folders immediately
    And empty projects left behind are pruned
    And NO scan is enqueued, because deleting needs no walk
    And the live watcher is re-registered so it stops reporting that subtree

  Scenario: An exclusion is removed
    When the excluded list loses an entry
    Then one ScanRoot with scope Full is enqueued
    And the newly un-excluded subtree is discovered and indexed

  Scenario: An exclusion is stored in a different case than the directory
    Given the filesystem is case-insensitive (macOS APFS, Windows NTFS)
    And the user stored the exclusion as "archive"
    And the directory on disk is "Archive"
    Then the subtree is STILL excluded
    # An exclusion is a privacy boundary; a byte-exact compare fails OPEN there.
    # pinned: scan_logic::tests::an_exclusion_matches_regardless_of_case_on_a_case_insensitive_filesystem

  Scenario: A root is removed
    When DELETE /api/workspace/roots/:id succeeds
    Then the folders_to_watch row is deleted
    And folders.root_id ON DELETE CASCADE removes its folders
    And nodes, edges and files cascade from those folders
    And no task is enqueued, because there is nothing left to scan
```

## Filesystem events

```gherkin
Feature: An event narrows the work rather than triggering a full scan

  Scenario: A source file is edited
    When the watcher debounces a batch containing one modified file
    Then ONE ScanRoot is enqueued for that file's watch root
    And its scope is Events with that path in `changed`
    And repo::narrow keeps only the repository owning it
    And reconcile_roots is SKIPPED, because the scope is not exhaustive
    And prune_vanished is SKIPPED for the same reason
    # pinned: root_watcher::tests::process_batch_enqueues_one_scoped_scan_per_watch_root
    #         repo_scan::scan_tests::an_event_scope_never_prunes_the_files_it_did_not_examine

  Scenario: A source file is deleted
    When the batch contains a Delete event
    Then the path lands in scope.deleted, not scope.changed
    And process_git_folder unresolves its inbound edges and drops its nodes
    And this happens even though the scope is NOT exhaustive,
      because a delete was OBSERVED rather than inferred from absence
    # pinned: repo_scan::scan_tests::an_observed_delete_is_applied_even_under_an_event_scope
    #         root_watcher::tests::process_batch_carries_deletes_apart_from_changes

  Scenario: A file changes in a repository created since the last scan
    Given the repository has no folder row
    When the batch arrives
    Then repo::discover finds it on DISK, not by lookup
    And it is registered and scanned like any other
    # This is why discovery reads the filesystem and never the database.
    # NO TEST — the disk-first property is pinned only at the unit level.

  Scenario: FSEvents drops events
    When notify reports need_rescan
    Then the affected roots are re-scanned with scope Full
    And with no path attached, ALL roots are re-scanned
    # pinned: root_watcher::tests::rescan_reconcile_roots_*

  Scenario: A branch is switched
    When .git/HEAD changes
    Then the REPOSITORY is re-scanned, not its watch root
    # One root here holds 67 repositories; one checkout must not re-walk 66 others.
    And the new branch is recorded in the typed folders.branch column

  Scenario: The watcher stalls silently
    Given the watch thread has delivered no event for 30 minutes
    Then it is marked unhealthy and surfaced on /api/watcher/status
    And a reconcile is forced, and the FSEvents stream re-established
```

## Safety

```gherkin
Feature: The index never deletes what it did not look at

  Scenario: A directory cannot be read
    Given ~/Documents is TCC-protected and the daemon lacks Full Disk Access
    When a full ScanRoot walks it
    Then read_dir returns "Operation not permitted"
    And the path is recorded in Discovered.unreadable
    And is_complete() returns false
    And reconcile_roots is SKIPPED
    # Otherwise every repository under it reads as deleted, and a removal cascades.
    # pinned: search::tests::an_unreadable_directory_is_reported_and_not_read_as_empty

  Scenario: An event batch mentions two files in a 400-file repository
    Then the other 398 keep their nodes, and prune_vanished does not run
    # pinned: repo_scan::scan_tests::an_event_scope_never_prunes_the_files_it_did_not_examine

  Scenario: A manifest is malformed
    When ProcessManifest fails to parse it
    Then the gate is RELEASED anyway
    And files whose placement came from it are skipped as unplaced — a visible gap
    # Holding it shut would strand every OTHER file behind one bad manifest.
    # pinned: repo_scan::tests::a_failed_manifest_does_not_deadlock_the_gate

  Scenario: A repository has no manifest
    Then blocked_by(vec![]) leaves the gate Pending and it runs immediately
    # pinned: repo_scan::tests::a_repo_with_no_manifests_opens_the_gate_at_once

  Scenario: The gate must wait for every manifest
    When enqueue_manifest_gate is called with two manifests
    Then the gate is BLOCKED until both complete
    # pinned: repo_scan::tests::enqueue_manifest_gate_blocks_the_gate_on_its_manifests

  Scenario: A file row cannot be written
    Then process_git_folder returns Err rather than Ok
    # A missing row means the gate never fans that file out — silently, while
    # expected_files still counts it.
```

## Classification and indexing

```gherkin
Feature: Every file examined is recorded, indexed or not

  Scenario Outline: One walk classifies everything
    When repo::scan passes over <file>
    Then it is classified <class> and <row> a files row

    Examples:
      | file              | class                | row      |
      | src/main.rs       | Source{Rust}         | gets     |
      | Cargo.toml        | Manifest{cargo}      | gets no  |
      | Cargo.lock        | Lockfile             | gets no  |
      | README.md         | Unsupported          | gets     |
      | pnpm-lock.yaml    | Unsupported          | gets     |
    # pinned: repo::tests::classify_names_manifests_lockfiles_source_and_unsupported

  Scenario: A lockfile no adapter can parse
    Given pnpm-lock.yaml, which npm's adapter does not list
    Then it is Unsupported, not Lockfile
    # Classification is a CAPABILITY claim; calling it a lockfile promises pins
    # that never arrive.

  Scenario: A claimed extension holds bytes the adapter cannot read
    Given a .rs file that is binary or not valid UTF-8
    Then classify_unscannable gives it BinaryContent or InvalidUtf8
    And no parse task is enqueued for it
    # A file with no row reads as changed for ever — the infinite re-index loop.

  Scenario: A source file is queued for parsing
    Then its files row sits on BARRIER_MTIME / BARRIER_HASH until a parse advances it
    # A row bearing its TRUE fingerprint reads as UNCHANGED next pass and is never indexed.
    # pinned: repo_scan::scan_tests::the_gate_fans_out_file_tasks_that_actually_index

  Scenario: A file is successfully parsed
    Then mark_file_parsed sets parsed_at
    And list_unparsed_files no longer returns it
    # Without this the gate re-fans every file on every scan, for ever.
    # pinned: repo_scan::scan_tests::the_gate_fans_out_file_tasks_that_actually_index

  Scenario: The cycle produces a graph
    When the gate's ProcessFile tasks run
    Then sensei.nodes is non-empty for that folder
    # The only assertion that distinguishes a working pipeline from one that
    # silently indexes nothing.
    # pinned: repo_scan::scan_tests::the_gate_fans_out_file_tasks_that_actually_index
```
