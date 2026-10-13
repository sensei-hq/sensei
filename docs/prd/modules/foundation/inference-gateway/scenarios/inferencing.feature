Feature: Inference Gateway — chain routing, fallback, consensus, budget, and the breaker

  # pinned: load_gateway_config_reads_embedded_first_from_db
  Scenario: gateway config loads from the embedded bundle first, then the database
    Given a daemon boot
    When the gateway config is loaded
    Then the embedded bundle is the initial source
    And the database can override it

  # pinned: the_four_catalog_queries_return_the_seeded_gateway_config
  Scenario: the gateway catalog is seeded and queryable
    Given a seeded gateway config
    When the four catalog queries are issued
    Then each returns the seeded chains, routers, and models

  # pinned: fastembed_through_gateway_returns_embeddings_for_a_real_model
  Scenario: embeddings flow through the gateway for the fast embedding model
    Given a completion pipeline with fastembed enabled
    When an embedding request is issued
    Then embeddings for a real model return through the gateway path

  # pinned: ort_through_gateway_returns_embeddings_for_a_real_model
  Scenario: embeddings also flow through the ONNX runtime path
    Given an ONNX-backed model
    When an embedding request is issued
    Then embeddings return through the gateway's ort path

  Scenario: the gateway routes each call to the chain suited to the task
    Given an offline-first default
    When a narration-cache call is issued
    Then it uses the narration-cache chain with ollama as primary and no remote fallback
    When a reasoning call is issued
    Then it may fall back to a remote chain when configured

  Scenario: a dead primary falls back gracefully and says so
    Given a primary provider that is down or times out
    When a call is issued
    Then the next provider in the fallback list is tried
    And if everything fails, an explicit error surfaces for a deterministic template fallback

  Scenario: the circuit breaker skips a failing endpoint for a cool-down
    Given an endpoint that has failed N times in the rolling window
    When further calls are issued
    Then the endpoint is skipped for the cool-down period
    And re-probed half-open before full service resumes

  Scenario: a budget hard cut routes to embedded ollama without blocking the caller
    Given a budget at 100%
    When a call is issued
    Then it routes to gateway-embedded ollama
    And the caller is never fully blocked

  Scenario: high-stakes calls run propose, challenge, synthesize
    Given a call flagged for consensus
    When propose runs two or three models in parallel
    Then each returns text and a reasoning trace
    When challenge shows each model the others' answers
    Then each returns agreements, disagreements, and a refined answer
    When synthesize combines them
    Then the result carries a confidence of high, medium, or low
    And the raw traces remain accessible for the reasoning panel

  Scenario: an offline machine stays fully functional
    Given a machine with no network
    When any chain is issued
    Then it resolves to the embedded model with a defined fallback path

  # gap: C-12
  Scenario: the inference pipeline review is scoped as a task
    Given the inference pipeline review from decision 10
    When it is scheduled
    Then it is scoped with the embedded-dep pin and the adapter seam as explicit review points