Feature: Metrics & Health — registry, values, roll-ups, health and gating

  # pinned: get_metrics_registry_endpoint
  Scenario: the registry is self-describing
    Given a seeded metric registry
    When the registry endpoint is read
    Then every active metric carries purpose, direction and formula

  # pinned: get_metric_status_endpoint
  Scenario: metric status is reported per repository
    Given a repository with stored values
    When status is queried for that repository
    Then computation state is reported from the stored values

  # pinned: get_metric_status_summary_endpoint
  Scenario: status summary covers the whole estate
    Given several repositories
    When the status summary is read
    Then each metric is judged live, thin or never — never by the registry alone

  # pinned: every_dimension_of_a_metric_row_reaches_its_own_column
  Scenario: every dimension of a metric row maps to its own column
    Given a stored metric row
    When it is read back
    Then metric, repository, scope, identity, grain and value land in their own columns

  # pinned: upsert_project_metric_is_idempotent
  Scenario: re-running a metric day never doubles history
    Given a metric day that was already written
    When the compute task runs again
    Then the row is upserted on the identity, not duplicated

  # pinned: re_writing_one_key_updates_the_row_rather_than_adding_another
  Scenario: rewriting a key updates the existing row
    Given an existing metric row
    When the same key is written again
    Then the row is updated rather than added to

  # pinned: session_outcomes_no_data_writes_zero_rows
  Scenario: no data means no row, never a fabricated zero
    Given a repository with no sessions in the window
    When the outcome metrics compute
    Then no rows are written

  # pinned: session_outcomes_writes_one_row_per_repository
  Scenario: outcome metrics write one row per repository
    Given a session in a repository
    When ftr, rework and throughput compute
    Then each metric writes a single row for that repository

  # pinned: ftr_parity_store_vs_views
  Scenario: FTR reads one number everywhere
    Given stored ftr values
    When the ftr endpoints and views are read
    Then store, views and API all agree

  # pinned: holistic_ftr_daily_pools_not_average_of_rates
  Scenario: a ratio rolls up from its parts, not from daily rates
    Given a week of daily ftr readings
    When the weekly roll-up runs
    Then it re-derives from summed numerators and denominators
    And is not the mean of the daily rates

  # pinned: active_metrics_excludes_retired_and_future
  Scenario: retired and future-dated metrics are not active
    Given a metric past its effective_until and a metric before its effective_from
    When the active set is computed
    Then neither is included

  # pinned: get_project_metrics_excludes_a_retired_metric
  Scenario: a retired metric never renders
    Given a retired metric
    When project metrics are read
    Then it is absent from the results

  # pinned: project_health_normalizes_by_direction_and_weight
  Scenario: the health score normalizes by direction and weight
    Given rated metrics with weights and directions
    When the health view is derived
    Then each metric contributes by direction and weight on 0–100

  # pinned: project_health_absent_when_no_components
  Scenario: a repository with nothing rated has no health row
    Given a repository with no rated metrics
    When the health view is derived
    Then no row exists — honest-empty, never a zero

  # pinned: project_health_count_normalization_and_target_null_excluded
  Scenario: counts normalize against target, and a null target excludes
    Given count and duration metrics
    When the health view is derived
    Then those without a target are excluded

  # pinned: project_health_neutral_direction_excluded
  Scenario: neutral descriptive metrics never score
    Given a neutral-direction metric
    When the health view is derived
    Then it is excluded from the score

  # pinned: project_health_retired_compute_writes_no_row
  Scenario: the retired health composite writes nothing
    Given the retired project_health composite in the registry
    When the health compute runs
    Then no stored health row is written
    And the score is still available as a derived view

  # pinned: metric_ratings_and_project_health_score_views
  Scenario: ratings and the health score are views over the store
    Given stored metric values
    When the ratings and health views are read
    Then both derive from the same value store

  # pinned: get_project_ftr_absent_is_null_not_zero
  Scenario: an absent metric reads as null, not zero
    Given a project with no ftr values
    When its ftr is fetched
    Then the value is null

  # pinned: no_fabricated_zero_ftr_in_metrics_surfaces
  Scenario: no metrics surface fabricates a zero
    Given a project with sparse data
    When every metrics surface renders
    Then no fabricated zero ftr appears anywhere

  # pinned: the_group_is_skipped_only_when_every_one_of_its_metrics_is_off
  Scenario: a compute group runs while any of its metrics survives
    Given a group whose metrics are mostly disabled
    When the group gate is evaluated
    Then the group still runs while one metric is alive
    And is skipped only when all are off

  # pinned: empty_active_registry_enqueues_nothing
  Scenario: an empty active registry schedules nothing
    Given a registry with no active metrics
    When the scheduler ticks
    Then nothing is enqueued

  # gap: M-7
  Scenario: every metric carries a master-detail record with gating eligibility
    Given the master-detail metric model
    When a metric's detail is inspected
    Then it declares whether, and how, it may gate (none | delta | ratchet)
    And the gating field is present in the registry seed

  # gap: M-8
  Scenario: thin metrics render behind a coverage gate
    Given a metric backed by few repositories
    When it is presented
    Then it is faded or gated, never drawn like a strong number

  # gap: M-9
  Scenario: every metric pairs with its chart and level of detail
    Given the per-metric design record
    When the design coverage is read
    Then the chart and intended level of detail are recorded

  # gap: D-6
  Scenario: defected metric captions and scales are not advertised
    Given a metric whose caption describes a retired feed
    And a metric whose rating scale is in the wrong unit
    When the registry is presented
    Then neither defects are silently repeated