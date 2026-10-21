Feature: Traceability & Doc Drift — scanning, confidence tiers, apply-fix, links

  # pinned: extracts_snake_and_pascal_multiword_identifiers
  Scenario: multi-word identifiers extract from doc prose
    Given a doc mentioning a snake_case and a PascalCase identifier
    When the extractor runs
    Then both identifiers are mentioned

  # pinned: rejects_single_word_bare_identifiers
  Scenario: a bare single word is not a code reference
    Given a doc mentioning a common single word in backticks
    When the extractor runs
    Then it is not emitted as an identifier mention

  # pinned: skips_stopwords_short_tokens_and_digits
  Scenario: stopwords, short tokens and digits never drift
    Given a doc mentioning TODO, short tokens and numbers
    When the extractor runs
    Then none of them become potential drift items

  # pinned: skips_fenced_code_blocks
  Scenario: fenced code blocks are skipped
    Given a doc with a fenced code block
    When the extractor runs
    Then the code inside the fence is not scanned for copy

  # pinned: strips_trailing_parens_from_function_call_mentions
  Scenario: trailing parens strip from call mentions
    Given `LoginService.authenticate()` in a doc
    When the mention is extracted
    Then the parens are stripped
    And the bare identifier is kept

  # pinned: deduplicates_repeated_mentions
  Scenario: repeated mentions within a doc deduplicate
    Given a doc mentioning one identifier several times
    When the extractor runs
    Then one mention entry results

  # pinned: is_broken_drift_flags_only_previously_indexed_now_missing
  Scenario: drift flags only identifiers that were indexed and are now missing
    Given an identifier that was previously in the code graph and is now gone
    When the drift scan runs
    Then it is flagged as broken
    And an identifier that never existed is not flagged

  # pinned: flags_broken_only_after_a_real_symbol_is_removed_then_resolves
  Scenario: a removed symbol flags, then resolves when it returns
    Given a symbol removed from the code
    When the scan runs
    Then the drift flags
    And when the symbol returns the flag resolves

  # pinned: rejects_bad_project_id_payload
  Scenario: a bad scan payload is rejected, not silently accepted
    Given a malformed workspace id in the scan request
    When the scan task runs
    Then it is rejected as an error

  # pinned: doc_references_resolve_to_files_and_unambiguous_symbols
  Scenario: doc references resolve to files and unambiguous symbols
    Given a doc referencing a file and a unique symbol
    When the reference pass runs
    Then both resolve

  # pinned: process_ir_creates_traceability_edge
  Scenario: doc processing creates a traceability edge
    Given a doc node and a referenced code node
    When the doc processor runs
    Then a traceability edge connects them

  # pinned: extract_rationale_matches_markers_at_word_boundary
  Scenario: rationale markers extract at word boundaries
    Given a doc with rationale markers
    When the rationale extractor runs
    Then the rationale nodes are emitted

  # pinned: extract_and_resolve_links
  Scenario: doc links extract and resolve
    Given a doc containing links
    When the link extractor runs
    Then the links are extracted and resolved

  # pinned: convergence_repairs_multiple_drift_classes_at_once
  Scenario: the index audit converges multiple drift classes
    Given a graph with several drift classes
    When the index audit runs
    Then all classes are repaired in one convergence pass

  # pinned: active_project_warns_on_drift_or_low_ftr
  Scenario: the workspace overview surfaces a drift stat
    Given a workspace with open drift items
    When the overview is composed
    Then the drift count warns

  # pinned: print_index_doctor_handles_clean_and_drift_reports
  Scenario: the CLI doctor reports clean and drift states
    Given a workspace that is clean and one with drift
    When the index doctor runs
    Then both states are reported correctly

  # gap: M4
  Scenario: requirement-level traceability renders section granularity
    Given section nodes in the code graph
    When drift is scanned at section level
    Then a doc section is matched to its code section
    And a drifted requirement carries expected and actual signature