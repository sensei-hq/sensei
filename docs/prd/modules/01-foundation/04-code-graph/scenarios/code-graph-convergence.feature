Feature: Code graph convergence — idempotent, scoped, durable indexing

  # pinned: graph_scan_end_to_end
  Scenario: an end-to-end scan produces a valid graph in one pass
    Given a fixture repo rooted at the graph-scan fixtures
    When a full scan completes
    Then nodes and edges are written and communities are assigned

  # pinned: a_second_scan_of_an_unchanged_repo_keeps_the_parse_state
  Scenario: re-scanning an unchanged repo changes zero rows
    Given a previously scanned fixture repo
    When it is scanned again with no changes
    Then the node id set is identical
    And edge counts per kind are identical
    And every node keeps its community_id

  # pinned: a_rescan_reparses_a_file_whose_content_changed
  Scenario: a changed file is re-parsed on rescan
    Given a file whose content changed
    When a rescan runs
    Then the file is re-parsed and its slice re-indexed

  # pinned: process_file_reindex_keeps_surviving_node_and_prunes_removed
  Scenario: a reindex keeps surviving symbols and prunes removed ones
    Given a file being reindexed
    When an earlier symbol survives
    Then it keeps its id and community_id
    When a symbol vanished from the file
    Then it is pruned

  # pinned: a_call_a_file_stopped_making_loses_its_edge
  Scenario: a call edge to a file that stopped exporting it is dropped
    Given an edge from A to B that A no longer produces
    When the graph re-resolves
    Then the stale edge is removed
    And the sampled node's other edges are untouched

  # pinned: doc_rationale_comment_emits_rationale_node
  Scenario: a rationale comment emits a rationale node
    Given a doc file with a rationale comment
    When the file is parsed
    Then a rationale node is emitted under the doc hierarchy

  # pinned: every_import_edge_carries_a_verdict_column
  Scenario: every import edge carries a verdict
    Given a scanned repo with imports
    When edges are written
    Then each import edge carries a verdict column value

  # pinned: dedup_structural_folder_nodes_removes_root_twins_keeps_uniques
  Scenario: structural folder nodes are deduplicated per folder
    Given a folder whose structural nodes duplicated on a previous pass
    When the reconcile runs
    Then root twins are removed and unique node rows are kept

  # pinned: audit_repairs_orphan_nodes
  Scenario: orphaned nodes are repaired by audit
    Given nodes left orphaned by an earlier failed pass
    When audit runs
    Then the orphans are repaired or named

  # pinned: convergence_repairs_multiple_drift_classes_at_once
  Scenario: one converge pass repairs multiple drift classes together
    Given a graph with several distinct kinds of drift
    When the converge pass runs
    Then each drift class is repaired in the same pass

  # pinned: reconcile_classifies_monorepo_member_roles
  Scenario: monorepo members are classified into roles
    Given a monorepo with package-workspace members
    When reconcile runs
    Then each member folder is classified into subtree or workspace_member

  Scenario: clear + full scan reaches the same steady state as incremental scans
    Given the same fixture
    When a clear-graph + full-scan completes
    Then its node id set, edge counts, and community assignment equal those reached by replaying the edits incrementally

  Scenario: restart mid-scan converges
    Given a scan interrupted by a simulated daemon restart
    When the daemon resumes
    Then running task executions reconcile to zero
    And the folder reaches indexed with the converged graph