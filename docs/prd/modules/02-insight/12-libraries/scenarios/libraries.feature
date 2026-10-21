Feature: Libraries — detect, wrap, query and watch

  # pinned: all_three_routes_agree_on_what_counts_as_a_doc
  Scenario: all ingestion routes agree on what counts as a doc
    Given llms.txt, repo-tree and published-API routes
    When a library is ingested through each
    Then all three agree on the doc pages

  # pinned: parse_llms_txt_sections
  Scenario: an llms-full.txt splits into per-component pages
    Given a library with an llms-full.txt
    When it is ingested
    Then sections split into component pages

  # pinned: the_full_text_sibling_is_derived_only_for_the_documented_pairing
  Scenario: a full-text sibling derives only for its documented pairing
    Given a component page with a designated full-text sibling
    When ingestion runs
    Then only that pairing derives a sibling

  # pinned: real_dbd_single_file_has_deploy_component
  Scenario: a single-file library maps components from its headings
    Given the real dbd docs as a single file
    When ingestion runs
    Then the deploy component is present

  # pinned: real_rokkit_per_file_has_list_component
  Scenario: a per-file library maps components across files
    Given the real rokkit docs spread across files
    When ingestion runs
    Then the List component is present

  # pinned: an_exact_choice_is_served_only_by_the_release_that_was_pinned
  Scenario: a pinned version is served only by the release that was pinned
    Given a library pinned to an exact version
    When a doc query resolves
    Then only the pinned release's docs are served

  # pinned: a_stated_version_is_preferred_over_an_unstated_one_even_from_a_better_route
  Scenario: a stated version beats a better route without one
    Given one route documenting the pinned version and a better-ranked route that does not
    When the doc query resolves
    Then the stated-version route wins

  # pinned: no_candidates_is_none_not_a_fabricated_choice
  Scenario: no matching candidates resolve to none, not a guess
    Given a query with no matching versioned docs
    When the docs resolve
    Then the result is none

  # pinned: load_manifest_resolves_paths_and_drops_unreadable
  Scenario: custom-library manifests load paths and drop unreadable ones
    Given a library with readable and unreadable manifest paths
    When the manifest loads
    Then the readable paths resolve
    And the unreadable ones are dropped

  # pinned: the_published_members_become_the_librarys_packages
  Scenario: published package-workspace members become the library's packages
    Given a package workspace with published and private members
    When the manifest groups them
    Then the published members become packages

  # pinned: a_private_member_is_not_a_package_anything_can_reference
  Scenario: a private member is not referenceable as a package
    Given a package workspace with a private member
    When packages are resolved
    Then the private member is not a package others may reference

  # pinned: tick_notifies_a_real_bump_and_dedupes
  Scenario: the update watcher notifies on a real version bump and dedupes
    Given a library whose pin has genuinely moved
    When the watcher ticks
    Then a notification fires
    And a repeated tick for the same bump does not duplicate

  # pinned: patch_bump_enqueues_reindex_not_a_rec
  Scenario: a patch bump enqueues a reindex, not a recommendation
    Given a patch-level version bump
    When the watcher reacts
    Then a reindex task is enqueued

  # pinned: security_bump_flags_high_urgency_refreshes_and_never_touches_pin
  Scenario: a security bump flags high urgency and never touches the pin
    Given a high-severity advisory on a pinned version
    When the watcher reacts
    Then the update flags high urgency
    And the pin is left untouched

  # pinned: patch_without_source_falls_back_to_notify
  Scenario: a bump without a doc source falls back to notify
    Given a version bump with no fetchable source
    When the watcher reacts
    Then it falls back to a notification

  # pinned: should_reindex_gate
  Scenario: reindexing is gated
    Given a pending update
    When the gate is evaluated
    Then reindexing is allowed only when appropriate

  # pinned: detect_stack_svelte
  Scenario: the stack detector recognises a Svelte workspace
    Given a workspace with a Svelte config
    When the stack is detected
    Then svelte is among the detected stacks

  # pinned: library_upsert_and_get
  Scenario: a detected library persists and reads back
    Given a detected library
    When it is upserted and fetched
    Then it round-trips

  # pinned: upsert_referenced_library_merges_props
  Scenario: re-detection merges props on a referenced library
    Given a library already referenced
    When it is upserted again
    Then its props merge rather than replace wholesale

  # pinned: replace_library_capabilities_is_manifest_authoritative
  Scenario: library capabilities are manifest-authoritative
    Given a library whose manifest declares capabilities
    When capabilities are replaced
    Then the manifest's declarations win

  # pinned: re_detect_preserves_the_model_authored_description
  Scenario: re-detection preserves a model-authored description
    Given a library with a model-authored description
    When it is re-detected
    Then the description is preserved

  # pinned: resolve_libs_is_fail_closed_on_a_failed_folder
  Scenario: resolution fails closed on a missing library folder
    Given a library folder that cannot be resolved
    When the library is resolved
    Then it fails closed rather than guessing

  # pinned: re_minting_an_unchanged_library_node_does_not_move_it
  Scenario: re-ingesting an unchanged library does not move its node
    Given a library re-ingested with unchanged content
    When the node is re-minted
    Then it stays at the same identity

  # pinned: library_skills_endpoints_serve_capabilities
  Scenario: generated skills are served from the skills endpoints
    Given a library above the skill threshold
    When the skills endpoints are read
    Then its capabilities are served