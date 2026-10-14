Feature: Insights & Impact — the verdict engine, narration and model insight

  # pinned: ftr_delta_buckets_verdicts_at_the_5pp_boundary
  Scenario: an FTR delta of ±5pp buckets the verdict
    Given a project with a baseline FTR and a current FTR
    When the delta is greater than +0.05
    Then the verdict is positive
    When the delta is less than -0.05
    Then the verdict is negative
    When the delta is within ±0.05
    Then the verdict is neutral

  # pinned: compose_headline
  Scenario: the headline composes for each verdict
    Given a positive verdict with a delta and a top model
    When the headline is composed
    Then it states the lift and the model
    Given a negative verdict with a delta
    When the headline is composed
    Then it states the regression
    Given a neutral verdict
    When the headline is composed
    Then it stays quiet

  # pinned: compose_body
  Scenario: the body composes for each verdict
    Given a positive verdict with baseline, current and a source memory
    When the body is composed
    Then it includes the delta and the source memory id
    Given a negative verdict
    When the body is composed
    Then it includes the delta and the suggested revision
    Given a neutral verdict
    When the body is composed
    Then it omits the revision

  # pinned: suggested_revision
  Scenario: a negative verdict gets a concrete revision
    Given a negative verdict with model effectiveness data
    When the revision is suggested
    Then it names a concrete alternative

  # pinned: synthesize_positive_includes_lift_headline_body_and_real_models
  Scenario: a positive synthesis includes lift, headline, body and real models
    Given a positive verdict with model data
    When the narration is synthesized
    Then it includes the lift headline, the body and the real models list

  # pinned: synthesize_negative_includes_regression_and_revision
  Scenario: a negative synthesis includes the regression and the revision
    Given a negative verdict
    When the narration is synthesized
    Then it includes the regression headline, the body and the revision

  # pinned: synthesize_neutral_stays_quiet_and_omits_revision
  Scenario: a neutral synthesis stays quiet and omits the revision
    Given a neutral verdict
    When the narration is synthesized
    Then the headline is quiet and there is no revision

  # pinned: no_models_captured_yields_an_empty_models_list_not_a_fabricated_vote
  Scenario: no models captured yields an empty models list, never a fabricated vote
    Given a verdict with no models captured
    When the narration is synthesized
    Then the models list is empty and no consensus is fabricated

  # pinned: synth_never_fabricates_a_consensus_panel
  Scenario: synthesis never fabricates a consensus panel
    Given any verdict
    When the narration is synthesized
    Then there is no fabricated consensus panel

  # pinned: recommends_clear_winner
  Scenario: model insight recommends a clear winner
    Given a project with model effectiveness data and a clear leader
    When model insight runs
    Then it recommends the winner

  # pinned: no_reco_when_lead_too_small
  Scenario: no recommendation when the lead is too small
    Given model data where the top two are close
    When model insight runs
    Then it makes no recommendation

  # pinned: no_reco_without_two_qualifying_models
  Scenario: no recommendation without two qualifying models
    Given fewer than two qualifying models
    When model insight runs
    Then it makes no recommendation

  # pinned: ignores_low_sample_models_in_ranking
  Scenario: low-sample models are ignored in the ranking
    Given a model with too few samples
    When model insight ranks
    Then that model is excluded

  # pinned: urgency_scales_with_gain
  Scenario: urgency scales with the FTR gain
    Given two positive verdicts with different deltas
    When urgency is assigned
    Then the larger gain gets higher urgency

  # pinned: normalize_model_merges_casing_and_spacing
  Scenario: model names are normalised by merging casing and spacing
    Given the same model named with different casing and spacing
    When model insight normalises
    Then they merge into one label

  # pinned: fold_model_stats_merges_label_variants_and_reweights
  Scenario: model stats merge label variants and reweight
    Given model stats for the same model under different labels
    When they are folded
    Then they merge and reweight by volume

  # pinned: fold_effectiveness_merges_and_orders_by_volume
  Scenario: effectiveness merges and orders by volume
    Given effectiveness records for the same model
    When they are folded
    Then they merge and order by volume

  # NO TEST (real test: facts_hash_is_order_independent): narration_cache_key_is_sha256_of_kind_plus_canonical_json
  Scenario: the narration cache key is sha256 of kind plus canonical json
    Given identical inputs in different orders
    When the cache key is computed
    Then the keys are equal
    Given inputs that differ
    When the cache key is computed
    Then the keys differ

  # NO TEST (real test: reinforce_source_memory_for_rec_bumps_promotes_and_is_idempotent): positive_verdict_idempotently_promotes_reinforce_memory
  Scenario: a positive verdict idempotently promotes a reinforce memory
    Given a positive verdict with a source memory
    When MeasureVerdicts runs twice
    Then exactly one reinforce memory is promoted

  # NO TEST (real test: measure_regressed_rec_challenges_source_memory_once): negative_verdict_idempotently_promotes_challenge_memory_with_revision
  Scenario: a negative verdict idempotently promotes a challenge memory with revision
    Given a negative verdict with a suggested revision
    When MeasureVerdicts runs twice
    Then exactly one challenge memory with that revision is promoted

  # NO TEST (no test exists — neutral verdict promotes nothing): neutral_verdict_promotes_no_memory
  Scenario: a neutral verdict promotes no memory
    Given a neutral verdict
    When MeasureVerdicts runs
    Then no memory is promoted