Feature: Memory & Knowledge — formation, scoping, lifecycle, retrieval

  # pinned: memory_create_and_get
  Scenario: a memory row round-trips
    Given a memory to save
    When it is created and fetched
    Then the same memory comes back

  # pinned: create_memory_persists_spine_slot_and_feature
  Scenario: a memory persists its spine, slot and feature
    Given a memory with a spine slot and feature
    When it is persisted
    Then spine, slot and feature survive

  # pinned: cluster_merges_similar_and_separates_distinct
  Scenario: correction evidence clusters into memory candidates
    Given similar correction signatures across sessions
    When the paper-clip pass runs
    Then the similar evidence merges into one candidate
    And distinct evidence stays separate

  # pinned: signature_is_deterministic_and_text_sensitive
  Scenario: the memory signature deduplicates deterministically
    Given the same memory text twice
    When signatures are derived
    Then they are identical
    And differing text yields different signatures

  # pinned: memory_reinforce
  Scenario: reinforcement advances a memory's support
    Given a proposed memory
    When it is reinforced
    Then its support grows

  # pinned: memory_reinforce_action_bumps_strength
  Scenario: the reinforce action bumps strength
    Given a memory behind the reinforce action
    When the action is applied
    Then the memory's strength increases

  # pinned: memory_challenge_action_sets_challenged
  Scenario: a challenge action marks a memory challenged
    Given a reinforced memory that is violated again
    When the challenge action is applied
    Then the memory is marked challenged

  # pinned: memory_archive_action_sets_archived
  Scenario: archiving marks a memory archived
    Given a challenged memory past its threshold
    When the archive action is applied
    Then the memory is marked archived

  # pinned: memory_dismiss_action_sets_rejected
  Scenario: dismissing marks a memory rejected
    Given a proposed memory the user dismisses
    When the dismiss action is applied
    Then the memory is marked rejected

  # pinned: memory_list_active
  Scenario: the active memory list filters by state
    Given memories across several states
    When the active list is read
    Then only active states appear

  # pinned: list_memories_for_slot_matches_slot_and_feature
  Scenario: memories are retrievable by slot and feature
    Given memories with known slots and features
    When retrieval filters by slot and feature
    Then only the matching memories are returned

  # pinned: assemble_context_blends_three_scopes
  Scenario: retrieval blends project, user and org scopes with the tighter one winning
    Given memories at project, user and org scope
    When the session context is assembled
    Then each scope contributes
    And the tighter scope wins on a signature conflict

  # pinned: assemble_context_leads_with_slot_anchored_memory
  Scenario: slot-anchored memory leads the context
    Given a memory anchored to the session's slot
    When the context is assembled
    Then it leads the returned list

  # pinned: assemble_context_logs_one_load_per_memory
  Scenario: every retrieval load is logged
    Given a session context assembly
    When memories are loaded
    Then each load is recorded in the load log

  # pinned: memory_loaded_last_7d_respects_window
  Scenario: load telemetry respects the seven-day window
    Given loads inside and outside the last seven days
    When the window is queried
    Then only the loads within seven days are counted

  # pinned: memory_followed_skipped_last_7d_over_outcomes
  Scenario: follow and skip evidence is counted from outcomes
    Given followed, skipped and violated reports
    When the seven-day counters are read
    Then followed and skipped are counted
    And violations are not treated as skips

  # pinned: project_learned_convention_scopes_to_its_own_project_not_general
  Scenario: a project-scoped memory never leaks to another project
    Given a learned convention in one project
    When another project's context is assembled
    Then the convention is absent

  # pinned: save_memory_rejects_a_secret_and_does_not_leak_it
  Scenario: a secret never lands in a memory
    Given a memory candidate containing a secret
    When it is saved
    Then it is rejected
    And the secret is not leaked

  # pinned: full_lifecycle_proposal_accept_outcome
  Scenario: a memory completes its full lifecycle
    Given a proposed memory
    When it is reviewed, accepted and its outcome reported
    Then the lifecycle completes end to end

  # pinned: promote_memory_prefers_project_over_cwd_folder
  Scenario: an explicit promotion resolves to the project scope
    Given a promotion request from a session
    When it is routed
    Then the project scope is preferred over the working folder

  # pinned: signature_is_order_independent_and_stable
  Scenario: consolidation merges by a stable, order-independent signature
    Given clusters being consolidated
    When merge signatures are computed
    Then the signature is stable regardless of member order

  # pinned: parse_clean_json
  Scenario: consolidation parses the merge proposal
    Given an assistant merge reply
    When it is parsed
    Then the actions are extracted

  # pinned: accept_as_rule_materializes_a_memory_and_records_provenance
  Scenario: a proven memory promotes to a rule with provenance
    Given a battle-tested memory promoted to a rule
    When the promotion lands
    Then a rule materializes from it
    And the provenance is recorded

  NO TEST
  Scenario: continuity memories auto-decay after seven days
    Given a continuity memory from a long session end
    When seven days pass
    Then it decays and is never promoted to a rule