Feature: Observatory — Insights, the three-column triage and its narration

  # pinned: rec_urgency_buckets
  Scenario: a recommendation buckets by its urgency
    Given recommendations of low, medium and high urgency
    When the board buckets them
    Then high is Now, medium is Soon and low is Settled

  # pinned: memory_violation_is_now_regardless_of_status
  Scenario: a violated memory is Now regardless of status
    Given a memory with a violation count above zero that is not archived
    When the board buckets it
    Then it lands in Now whatever its status

  # pinned: memory_status_buckets_when_not_violated
  Scenario: an unviolated memory buckets by status
    Given a memory with no violations
    When the board buckets it
    Then its column follows its status

  # pinned: pattern_lifecycle_buckets
  Scenario: a pattern buckets by its lifecycle
    Given a pattern with a lifecycle state
    When the board buckets it
    Then its column follows the lifecycle

  # pinned: rec_copy_inputs_maps_kind_facts_and_fallback
  Scenario: recommendation copy maps kind, facts and fallback
    Given a recommendation of a given kind
    When its copy inputs are built
    Then the kind, the facts and a fallback are mapped

  # pinned: rec_copy_inputs_is_deterministic_and_order_independent
  Scenario: copy inputs are deterministic and order-independent
    Given the same recommendation facts in two orders
    When the copy inputs are built
    Then both produce the same input

  # pinned: rec_copy_inputs_shared_cache_across_both_endpoints
  Scenario: one narration entry serves both endpoints
    Given a recommendation surfaced on two endpoints
    When its copy is warmed
    Then one cache entry serves both

  # pinned: rec_copy_inputs_null_impact_is_stable_and_distinct
  Scenario: a null impact has a stable and distinct copy input
    Given two recommendations with null impact
    When their copy inputs are built
    Then the inputs are stable and distinct from each other

  # pinned: apply_rec_copy_overwrites_only_title_and_why_preserving_wire_shape
  Scenario: applying copy overwrites only title and why
    Given a warmed narration
    When the copy is applied to the wire shape
    Then only title and why change and the shape is preserved

  # pinned: recommendation_lifecycle
  Scenario: a recommendation moves from pending to accepted or dismissed
    Given a pending recommendation
    When it is accepted
    Then its status becomes accepted
    When a separate pending recommendation is dismissed
    Then its status becomes dismissed

  NO TEST
  Scenario: the Insights Reasoning drawer explains one recommendation
    Given a recommendation with a reasoning trace
    When the reasoning drawer opens
    Then it shows one honest verdict with no fabricated consensus
