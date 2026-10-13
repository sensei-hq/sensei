Feature: Observatory — Today, the hero koan, and the early/mature mood switch

  # pinned: greeting_buckets
  Scenario: the greeting follows the time of day
    Given a request at a given local hour
    When the Today greeting is composed
    Then it lands in the matching time-of-day bucket

  # pinned: early_hero_shows_real_count_and_no_action
  Scenario: the early hero shows a real count and offers no action
    Given a project with too little signal for maturity
    When Today is composed
    Then the hero shows the real session count and no action button

  # pinned: early_hero_singular_session_word_and_min_remaining
  Scenario: the early hero uses the singular word and the minimum remaining
    Given exactly one watched session and a target above it
    When Today is composed
    Then the copy uses the singular session word and reports the minimum remaining

  # pinned: mature_hero_koan_is_rec_title_and_action_is_nav_cta
  Scenario: the mature hero is the recommendation title with a navigation CTA
    Given a mature project with a top recommendation
    When Today is composed
    Then the koan is the recommendation title and the action is a navigation CTA

  # pinned: mature_hero_null_impact_and_empty_source_when_absent
  Scenario: a mature hero with no impact and no source stays honest
    Given a mature project whose recommendation has no impact and no source
    When Today is composed
    Then the impact is null and the source is empty, not fabricated

  # pinned: insight_tone_maps_from_urgency
  Scenario: insight tone maps from urgency
    Given an insight of a given urgency
    When its tone is chosen
    Then the tone matches the urgency

  # pinned: insight_card_uses_title_and_urgency
  Scenario: an insight card renders its title and urgency
    Given an insight with a title and an urgency
    When its card is built
    Then the card carries the title and the urgency

  # pinned: early_insights_are_a_pair
  Scenario: the early state shows an insight pair
    Given a project still listening
    When supporting insights are chosen
    Then at most a pair is shown

  # pinned: adopted_row_maps_and_trims
  Scenario: an adopted row maps and trims to five
    Given more than five in-force memories
    When the adopted lane is built
    Then rows map to the lane and the list trims to five

  # pinned: evidence_uuid_extraction_walks_nested_json_and_caps
  Scenario: evidence uuids are extracted from nested json and capped
    Given a source blob with uuids nested in json
    When evidence is extracted
    Then the uuids are found and the list is capped

  # pinned: early_below_target_even_with_insights
  Scenario: below the target is early even with insights
    Given a project below the maturity target that has insights
    When maturity is decided
    Then it is early

  # pinned: early_at_target_without_insights
  Scenario: at the target with no insights is early
    Given a project at the maturity target with no insights
    When maturity is decided
    Then it is early

  # pinned: mature_at_target_with_insights
  Scenario: at the target with insights is mature
    Given a project at the maturity target that has insights
    When maturity is decided
    Then it is mature

  # pinned: fresh_project_is_early
  Scenario: a fresh project is early
    Given a project with no watched sessions
    When maturity is decided
    Then it is early

  # pinned: no_fabricated_zero_ftr_in_metrics_surfaces
  Scenario: an absent FTR is null, never a fabricated zero
    Given a project with no FTR reading
    When a metrics surface is composed
    Then FTR is null rather than zero

  NO TEST
  Scenario: Consolidation is reachable from the Observatory
    Given the Consolidation surface
    When the Observatory navigates to it
    Then the consolidation candidates render
