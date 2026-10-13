Feature: Patterns & Conventions — five sources, the MCP surface, anti-patterns

  # pinned: pattern_upsert_and_list
  Scenario: a detected pattern persists and lists
    Given a pattern to record
    When it is upserted and listed
    Then it comes back in the list

  # pinned: pattern_upsert_updates_existing
  Scenario: re-detecting a pattern updates it in place
    Given an existing pattern row
    When the pattern is upserted again
    Then the existing row is updated

  # pinned: pattern_upsert_merges_across_folders_in_same_project
  Scenario: instances merge across folders within a project
    Given the same pattern detected in different folders of one project
    When it is upserted
    Then the instances merge

  # pinned: accept_promote_pattern_advances_lifecycle_to_rule
  Scenario: accepting a promotion advances a pattern to a rule
    Given a detected pattern with a promotion
    When the acceptance is applied
    Then the pattern advances its lifecycle to rule

  # pinned: accept_promote_pattern_guard_fires_promotion_once
  Scenario: a promotion fires only once
    Given a pattern already promoted
    When acceptance is applied again
    Then the promotion guard prevents a second firing

  # pinned: accept_promote_pattern_reads_back_as_adopted
  Scenario: a promoted pattern reads back as adopted
    Given a promoted pattern
    When it is read back
    Then its state is adopted

  # pinned: ftr_delta_positive_when_folder_beats_baseline
  Scenario: a folder that beats baseline earns a positive ftr delta
    Given a folder whose ftr outperforms the project baseline
    When pattern effectiveness is scored
    Then the delta is positive

  # pinned: ftr_delta_negative_for_underperforming_folder
  Scenario: a folder under baseline earns a negative ftr delta
    Given a folder whose ftr trails the project baseline
    When pattern effectiveness is scored
    Then the delta is negative

  # pinned: kind_maps_anti_adopted_emerging
  Scenario: pattern modes map to design, anti and opt
    Given pattern kinds
    When they are mapped
    Then anti, adopted and emerging map to their modes

  # pinned: derive_conventions_reports_dominant_naming_per_kind
  Scenario: project conventions report the dominant naming per kind
    Given a project's symbols
    When conventions are derived
    Then the dominant naming convention per kind is reported

  # pinned: derive_conventions_empty_is_safe
  Scenario: an empty project yields safe conventions
    Given a project with no symbols
    When conventions are derived
    Then an empty, safe result is returned

  # pinned: get_patterns_proxy_aliases_to_get_file_tags_and_renames_pattern_to_tag
  Scenario: the get_patterns tool proxies to the file-tags surface
    Given a project with tagged files
    When get_patterns is called
    Then it proxies to the tag surface
    And the pattern parameter is carried as the tag

  # pinned: match_pattern_and_pattern_for_carry_their_args
  Scenario: match and by-name lookups carry their arguments
    Given a lookup request
    When the MCP tools are invoked
    Then the query and name arguments are passed through

  # gap: M10
  Scenario: codebase design patterns are detected from the graph
    Given a project whose code graph has > 1000 nodes
    When the codebase recognisers run
    Then design patterns present in the code are detected

  # gap: M10
  Scenario: library patterns are ingested when a library is added
    Given a library with a patterns section
    When add_library ingests it
    Then its patterns are indexed for match_pattern

  # gap: M10
  Scenario: an industry registry is reachable and queryable
    Given a registered registry source
    When match_pattern queries it
    Then patterns.dev-style entries resolve

  # gap: M10
  Scenario: derived patterns auto-promote after consistent usage
    Given a practice used consistently N times
    When the derived pass evaluates it
    Then it surfaces as a derived pattern
    And can be auto-promoted with a user override

  # gap: M10
  Scenario: an architectural option is stored and read back, never re-asked
    Given a chosen architectural option
    When the next session starts
    Then the choice is read back from pattern_choices
    And the assistant is not asked again