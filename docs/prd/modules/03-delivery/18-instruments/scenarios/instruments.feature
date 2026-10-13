Feature: Instruments — Playground, Replay and the verdict classifier

  # pinned: each_call_is_paired_with_its_own_response_and_the_engines_duration
  Scenario: a call pairs with its own response and the engine's duration
    Given a captured call and its response
    When the replay timeline is built
    Then the call is paired with its own response and the engine's duration

  # pinned: a_duplicated_capture_is_one_call_and_a_missing_id_is_unpaired
  Scenario: a duplicated capture is one call and a missing id is unpaired
    Given a capture duplicated and a call with no response id
    When the replay timeline is built
    Then the duplicate is one call and the missing id stays unpaired

  # pinned: a_verdict_shows_on_its_own_call_in_the_replay_timeline
  Scenario: a verdict shows on its own call
    Given a session with several calls and one verdict
    When the replay timeline is built
    Then the verdict shows on the call that earned it

  # pinned: a_failed_call_is_paired_with_its_failure
  Scenario: a failed call is paired with its failure
    Given a call that failed
    When the replay timeline is built
    Then the failure is paired with the call

  # pinned: classify_used_when_reaction_cites_a_fragment
  Scenario: a cited fragment is used
    Given a call whose reaction cites a fragment of the target
    When it is classified
    Then the verdict is used

  # pinned: classify_partial_when_same_target_but_no_citation
  Scenario: the same target with no citation is partial
    Given a call whose reaction names the same target but cites nothing
    When it is classified
    Then the verdict is partial

  # pinned: classify_ignored_when_next_event_is_user_prompt
  Scenario: a user prompt next is ignored
    Given a call followed immediately by a user prompt
    When it is classified
    Then the verdict is ignored

  # pinned: classify_ignored_when_targets_differ_and_no_citation
  Scenario: differing targets with no citation is ignored
    Given a call whose reaction names a different target and cites nothing
    When it is classified
    Then the verdict is ignored

  # pinned: classify_ignored_when_no_reaction
  Scenario: no reaction is ignored
    Given a call with no reaction
    When it is classified
    Then the verdict is ignored

  # pinned: primary_target_reads_common_fields
  Scenario: the primary target is read from the common fields
    Given a call payload with common target fields
    When the primary target is read
    Then it comes from those fields

  # pinned: extract_fragments_pulls_path_like_tokens
  Scenario: fragments pull path-like tokens
    Given reaction text with path-like tokens
    When fragments are extracted
    Then the path-like tokens are returned

  # pinned: extract_fragments_grabs_backtick_identifiers
  Scenario: fragments grab backtick identifiers
    Given reaction text with backtick identifiers
    When fragments are extracted
    Then the identifiers are returned

  # pinned: extract_fragments_skips_short_common_words
  Scenario: fragments skip short common words
    Given reaction text with short common words
    When fragments are extracted
    Then they are skipped

  # pinned: extract_response_text_handles_string_variant
  Scenario: response text is read from a string variant
    Given a response whose text is a plain string
    When the text is extracted
    Then it is returned

  # pinned: extract_response_text_handles_object_content
  Scenario: response text is read from object content
    Given a response whose content is an object
    When the text is extracted
    Then it is returned

  # pinned: extract_response_text_handles_array_of_text_parts
  Scenario: response text is read from an array of text parts
    Given a response whose content is an array of text parts
    When the text is extracted
    Then it is joined

  # pinned: extract_response_text_none_when_missing
  Scenario: missing response text is none
    Given a response with no text
    When the text is extracted
    Then it is none

  # pinned: used_confidence_climbs_with_more_hits
  Scenario: confidence climbs with more hits
    Given a reaction with several fragment hits
    When the confidence is computed
    Then it is higher than with one hit

  # pinned: serializes_variant_lowercase_and_optional_action
  Scenario: a signal serialises its variant lowercase
    Given a signal with a variant and an optional action
    When it is serialised
    Then the variant is lowercase and the action is optional
