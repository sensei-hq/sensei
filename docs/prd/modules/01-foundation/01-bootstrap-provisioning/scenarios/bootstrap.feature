Feature: Bootstrap — the six-gate health check that provisions the machine and hands over

  # pinned: builds_a_checker_for_every_component_id
  Scenario: every component has a health-check gate
    Given the bootstrap component registry
    Then each component id maps to a checker that names its programming probed path

  # pinned: provides_five_resolvers
  Scenario: the daemon offers one resolver per provisionable component
    Given the resolver registry
    Then each resolvable component has exactly one resolver

  # pinned: check_and_resolve_emits_initial_report_before_resolving
  Scenario: the machine reports the initial check report before any installation begins
    Given a machine with missing components
    When the check-and-resolve pass starts
    Then an initial report is emitted
    And remediation begins only after that report is observable

  # pinned: dependent_resolver_skipped_when_dependency_still_failing
  Scenario: a resolver does not run on a broken dependency
    Given a dependency that is still failing
    When the resolve pass schedules its dependents
    Then the dependent resolver is skipped for this pass

  # pinned: dependent_resolver_runs_when_upstream_recovers_in_same_pass
  Scenario: a resolver runs once its dependency recovers within the same pass
    Given a dependency that recovers mid-pass
    Then its dependent resolver runs in the same pass

  # pinned: detect_sets_model_tier
  Scenario: hardware detection names a model tier from the machine's RAM
    Given the hardware inspector
    When the machine's RAM and engine state are inspected
    Then a model tier of lite, balanced, advanced, or no-inference is detected

  # pinned: every_tier_provisions_the_embedded_defaults
  Scenario: every detected tier provisions its embedded default models
    Given any detected model tier
    Then the tier's embedded defaults are provisionable

  # pinned: missing_models_when_nothing_installed
  Scenario: a bare machine reports no models rather than silently empty
    Given a machine with nothing installed
    Then the model inventory reports the missing set explicitly

  Scenario: an all-green run hands over to the first-run scan within a bounded delay
    Given every gate in GET /api/bootstrap/status reports pass
    When the green state has rendered
    Then the splash transitions within 800ms
    And the Continue link is present in case the transition stalls

  Scenario: an offline machine degrades gracefully
    Given a machine with no network
    When the Homebrew and Ollama gates run
    Then they fall to remedy with a "run this yourself" note
    And the overall run continues rather than failing outright

  Scenario: Re-check from the Observatory returns to the Observatory, not first-run
    Given bootstrap was triggered via Re-check from the Observatory
    When all gates pass
    Then the green state closes back into the Observatory main window

  # gap: M1
  Scenario: app-only Tauri sidecar bootstrap reaches the same green as the daemon path
    Given the app runs without the full daemon stack
    When the sidecar bootstrap completes
    Then the same six gates report pass