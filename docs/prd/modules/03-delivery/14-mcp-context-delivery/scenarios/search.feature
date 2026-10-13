Feature: MCP & Context Delivery — hybrid search, ranking and the grep fallback

  # pinned: semantic_search_nodes_ranks_by_cosine
  Scenario: semantic search ranks nodes by cosine distance
    Given a node corpus with embeddings
    When a semantic search runs
    Then the nearest nodes rank first by cosine

  # pinned: semantic_search_nodes_drops_neighbours_beyond_the_distance_bound
  Scenario: neighbours beyond the distance bound are dropped
    Given a semantic search with a distance bound
    When it runs
    Then neighbours beyond the bound are dropped

  # pinned: fuse_ranks_shared_hit_first_and_dedupes
  Scenario: fusion ranks a shared hit first and dedupes
    Given a hit shared by two arms
    When the arms are fused
    Then the shared hit ranks first and is not duplicated

  # pinned: fuse_empty_semantic_preserves_lexical_order
  Scenario: an empty semantic arm preserves lexical order
    Given no semantic hits and a lexical ranking
    When the arms are fused
    Then the lexical order is preserved

  # pinned: fuse_empty_lexical_uses_semantic_order
  Scenario: an empty lexical arm uses semantic order
    Given no lexical hits and a semantic ranking
    When the arms are fused
    Then the semantic order is used

  # pinned: exact_name_match_is_promoted_above_prefix_and_concept_hits
  Scenario: an exact name match is promoted above weaker hits
    Given an exact name match, a prefix hit and a concept hit
    When the results rank
    Then the exact name match is promoted above the others

  # pinned: a_substring_collision_is_dropped_but_an_exact_name_match_is_never
  Scenario: a substring collision is dropped, an exact match is never dropped
    Given a substring collision and a simultaneous exact-name match
    When the results are filtered
    Then the collision is dropped and the exact match is kept

  # pinned: an_unscoreable_lexical_hit_is_kept_rather_than_judged
  Scenario: an unscoreable lexical hit is kept rather than judged
    Given a lexical hit that cannot be scored
    When the results are filtered
    Then it is kept rather than dropped

  # pinned: content_grep_is_case_insensitive_bounded_and_excludes
  Scenario: content grep is case-insensitive, bounded and excluding
    Given a content grep over a project
    When it runs
    Then it matches case-insensitively, is bounded, and honours exclusions

  # pinned: extract_snippet_clamps_range_and_caps
  Scenario: the extracted snippet clamps its range and caps its length
    Given a match near a boundary
    When the snippet is extracted
    Then the range is clamped and the length capped

  # pinned: to_hits_keys_by_id_and_drops_idless
  Scenario: hits are keyed by id and id-less hits are dropped
    Given a set of raw results, some without ids
    When they are converted to hits
    Then they are keyed by id and the id-less ones are dropped

  # pinned: embed_text_combines_kind_name_signature_and_path
  Scenario: embed text combines kind, name, signature and path
    Given a code node with kind, name, signature and path
    When its embed text is built
    Then it combines all four

  # pinned: cap_embed_input_truncates_overlong_ascii_under_ubatch
  Scenario: over-long embed input is truncated under the micro-batch
    Given an over-long ASCII text
    When the embed input is capped
    Then it is truncated to fit under the micro-batch

  # pinned: pack_worst_case_max_length_texts_never_exceed_ubatch
  Scenario: worst-case packing never exceeds the micro-batch
    Given a batch of maximum-length texts
    When they are packed
    Then no micro-batch exceeds the limit

  NO TEST
  Scenario: a none-confidence result carries a grep fallback hint
    Given a search whose combined confidence is none
    When the response is returned
    Then it carries a fallback hint and the caller reruns grep

  NO TEST
  Scenario: thin graph coverage surfaces a coverage-gap hint, not an empty answer
    Given a project whose edges are thin
    When a structural search finds nothing
    Then the response says coverage is thin rather than "no callers"
