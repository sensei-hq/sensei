---
id: system-intelligence
type: feature
---

# System Intelligence

> Sensei understands your entire system — not just individual repos — and surfaces gaps where reality deviates from design

Individual repo intelligence only tells part of the story. A payment platform may have a Go API service, a TypeScript dashboard, a React Native mobile app, a Python data service, a Kafka event bus, a shared Protobuf schema registry, and an infrastructure repo — each owned by a different team. The API team renames a field. The dashboard team doesn't know. The architecture team mandated a consistent error response format. Three of seven services deviate. Nobody has visibility across the whole until something breaks in production.

System Intelligence is **Layer 3** in sensei's layered design:

```
Layer 3: System Intelligence   — workspace, cross-repo graph, system gap analysis, conformance
              builds on ↓
Layer 2: Documentation & Traceability — per-repo traceability, drift, quality metrics, generated docs
              builds on ↓
Layer 1: Codebase Intelligence — per-repo scan, parse, index, symbols, call graph, external doc adapters
```

Each layer composes the one below — it never re-implements it. The workspace index is an orchestrator over per-repo indexers (Layer 1). Cross-repo traceability extends per-repo traceability (Layer 2) across repo boundaries. System health aggregates per-repo quality scores (Layer 2) with cross-repo metrics. Architecture conformance checks Layer 1 patterns against system docs indexed by Layer 1's external doc adapters.

This means every capability that improves Layer 1 or Layer 2 automatically improves Layer 3. Better language adapters → richer cross-repo symbol graphs. Better quality metrics → more accurate system health scores. Better generated docs → more complete system architecture coverage.

---

## Features

### Workspace Definition

A workspace is a named collection of repos and their roles that together form a system. Roles span the full spectrum of modern system decomposition: UI, backend, microservice, data, infra, shared-lib, gateway, schema-registry. System-level docs are declared alongside the repos. The workspace is stored in `sensei.workspaces` and orchestrates Layer 1 indexing across all member repos.

```gherkin
Feature: Workspace Definition

  Scenario: Developer defines a workspace from multiple repos
    Given repos: payment-api (Go), payment-dashboard (TypeScript), payment-mobile (React Native), payment-data (Python), payment-infra (Terraform), payment-schemas (Protobuf)
    When the developer runs sensei workspace init --name payment-platform
    And adds each repo: sensei workspace add ./payment-api --role microservice
    Then each repo is registered in sensei.workspace_repos with its role
    And the workspace is accessible from any member repo's get_session_context()

  Scenario: Workspace roles reflect microservices decomposition
    Given a microservices system
    Then supported roles include: ui, backend, microservice, data, gateway, schema-registry, shared-lib, infra, mobile
    And a repo can have multiple roles (e.g., a service that is both backend and schema-registry)

  Scenario: System docs are registered with the workspace
    Given a workspace with architecture docs in Confluence space PAYMENT-ARCH and local ADRs in ../payment-docs/decisions/
    When the developer registers both sources
    Then both are indexed via Layer 1's external doc adapters (Confluence adapter, local file adapter)
    And the indexed content is stored in sensei.system_docs, distinct from per-repo doc_sections
    And it is available to all repos in the workspace for context delivery and conformance checks

  Scenario: Developer context includes workspace awareness
    Given a workspace payment-platform with 7 repos
    When a developer opens payment-api in their IDE and calls get_session_context()
    Then the orientation includes: workspace name, this repo's role, sibling service names and roles, system doc summary, and any open cross-repo drift items affecting this repo
```

---

### Cross-Repo Indexing

Cross-repo indexing orchestrates Layer 1 indexers across all workspace repos and then builds a workspace-level graph on top of the per-repo symbol indexes. It resolves cross-repo references by contract type — REST (OpenAPI), GraphQL, gRPC (Protobuf), event schemas (AsyncAPI, Avro), shared types, and shared database schemas. The result is stored in `sensei.workspace_edges` and is queryable like any other part of the index.

Microservices communicate through implicit contracts — a service publishing to a Kafka topic doesn't import the consumer. Sensei resolves these async dependencies through topic name matching, schema registry references, and event schema analysis.

```gherkin
Feature: Cross-Repo Indexing

  Scenario: OpenAPI contract links backend service to consumers
    Given payment-api exposes openapi.yaml
    And payment-dashboard calls /api/v1/payments/process in 4 components
    And payment-mobile calls the same endpoint in 2 screens
    When the workspace index runs
    Then sensei.workspace_edges records consuming call sites in both repos
    And the endpoint is flagged if its contract changes

  Scenario: Protobuf schema links gRPC services
    Given payment-schemas defines PaymentService.proto
    And payment-api implements the server
    And payment-data implements the client
    When the workspace index runs
    Then workspace_edges record: payment-schemas → payment-api (server), payment-schemas → payment-data (client)
    And any .proto change triggers cross-repo drift evaluation

  Scenario: Kafka topic links async producer and consumer
    Given payment-api publishes to topic "payments.completed"
    And payment-data subscribes to "payments.completed"
    And no direct import relationship exists between the two services
    When the workspace index runs
    Then sensei resolves the implicit dependency via topic name matching and AsyncAPI schema
    And a workspace_edge is created: payment-api → payment-data via topic "payments.completed"

  Scenario: Shared database schema is tracked as a cross-repo contract
    Given payment-api and payment-reporting both read from the payments table in a shared database
    When the workspace index runs
    Then a workspace_edge is recorded: payments_db.payments → payment-api and payments_db.payments → payment-reporting
    And schema migrations in the DB repo are evaluated for impact on both consumers

  Scenario: Heterogeneous stacks index without conflict
    Given Go, TypeScript, Python, and Terraform repos in one workspace
    When the workspace index runs
    Then each repo uses its Layer 1 language adapter independently
    And cross-repo edges are resolved by contract boundaries, not by shared imports
    And the unified workspace graph is queryable regardless of per-repo language
```

---

### Service Map

The service map is a first-class workspace artifact — a live graph of all services, their roles, their contracts (what they produce and consume), and their dependencies. It is generated from the workspace index and updated on every index run. It is the system-level equivalent of the per-repo call graph, and it is the foundation for conformance checking, gap analysis, and system health scoring.

The service map is stored in `sensei.service_map` and rendered as a Mermaid diagram in the workspace dashboard. It replaces hand-drawn architecture diagrams that go stale the moment they are created.

```gherkin
Feature: Service Map

  Scenario: Service map is generated from workspace index
    Given a workspace with 7 repos indexed
    When the workspace index completes
    Then sensei.service_map is populated with: nodes (one per service), edges (one per contract boundary), and metadata (contract type, version, direction)
    And the map reflects the current actual state of the codebase, not a manually drawn diagram

  Scenario: Service map renders as Mermaid in dashboard
    Given a workspace with service map data in sensei.service_map
    When a developer opens the Workspace Dashboard
    Then a Mermaid diagram renders showing all services as nodes and contracts as labelled edges
    And async dependencies (Kafka, SQS) are shown with dashed edges to distinguish from synchronous calls
    And each node is clickable to navigate to that repo's detail view

  Scenario: Service map highlights drift and gaps
    Given the service map has 2 open cross-repo drift items and 1 missing service
    When the developer views the service map
    Then drifted edges are shown in red
    And the gap (designed service not yet implemented) is shown as a dashed node
    And hovering over a drifted edge shows the drift description

  Scenario: Service map is versioned over time
    Given service map snapshots stored on each workspace index run
    When the developer selects a date range in the dashboard
    Then they can see how the service map evolved over that period
    And services added, removed, or reconnected are highlighted
```

---

### Architecture Conformance

Architecture conformance checks each repo's actual implementation patterns against system-level standards declared in the workspace's system docs. Conformance rules are extracted from system docs by the local model (e.g., "all services must use structured JSON logging") and then evaluated against Layer 1's per-repo symbol and pattern analysis. This builds directly on Layer 2's quality analysis — adding the system-level reference standard that per-repo quality checks lack.

```gherkin
Feature: Architecture Conformance

  Scenario: Conformance rules are extracted from architecture docs
    Given a system architecture doc stating: "All public API endpoints must validate requests against a JSON schema before processing"
    When sensei processes the system docs
    Then a conformance rule is created: type "request-validation", scope "public-endpoint-handlers"
    And the rule is stored in sensei.conformance_rules with its source doc reference

  Scenario: Per-service conformance is evaluated against extracted rules
    Given a conformance rule requiring structured JSON logging
    And 7 microservices in the workspace
    When the conformance check runs
    Then each service's logging calls are analysed using Layer 1's call graph
    And services using unstructured logging (fmt.Println, console.log) are flagged as non-conformant
    And services using structured loggers (zap, winston, structlog) are marked as passing

  Scenario: Conformance deviation is linked to the source architecture doc
    Given payment-api is non-conformant on the request-validation rule
    When the developer views the conformance report
    Then the violation includes: service name, file, line, rule description, and a link to the architecture doc section that mandates it
    And the developer can navigate directly to the relevant architecture doc from the report

  Scenario: Workspace conformance score improves as teams fix violations
    Given a workspace conformance score of 62% across 7 services
    When two teams resolve their violations and sensei workspace index runs
    Then the conformance score increases to 78%
    And the improvement is recorded in the quality_reports time series
```

---

### Cross-Repo Drift Detection

Cross-repo drift extends Layer 2's per-repo drift detection across repo boundaries. When a contract changes in one repo — an API endpoint renamed, a Protobuf field removed, a Kafka topic schema updated, a shared type modified — all downstream consumers in the workspace are evaluated for compatibility. This builds directly on Layer 2 traceability and workspace_edges from cross-repo indexing.

```gherkin
Feature: Cross-Repo Drift Detection

  Scenario: REST endpoint rename causes cross-repo drift
    Given payment-dashboard calls POST /api/v1/payments/process
    And payment-api renames the endpoint to POST /api/v2/payments/initiate
    When the workspace drift check runs
    Then a cross-repo drift item is created: source payment-api, consumer payment-dashboard, contract REST, old path /v1/payments/process, new path /v2/payments/initiate
    And affected call sites in payment-dashboard are listed with file and line references

  Scenario: Protobuf field removal causes cross-repo drift
    Given payment-schemas removes field currency_code from PaymentRequest
    And payment-api and payment-mobile both set this field
    When the workspace drift check runs
    Then two drift items are created — one per consuming repo
    And each item references the specific file and line where the removed field is accessed

  Scenario: Async schema change causes cross-repo drift
    Given payment-api changes the PaymentCompleted event schema: renames amount_cents to amount_minor_units
    And payment-data expects amount_cents in its consumer handler
    When the workspace drift check runs
    Then a drift item is created between payment-api (producer) and payment-data (consumer)
    And the schema version mismatch and affected field are recorded

  Scenario: GitHub App notifies affected teams on PR
    Given the GitHub App is connected to the workspace
    When a PR in payment-api modifies an OpenAPI endpoint consumed by payment-dashboard (3 sites) and payment-mobile (1 site)
    Then the GitHub App posts a PR comment listing all consuming repos and call sites
    And the PR check status is set to "warning: cross-repo impact detected"
    And team leads of the affected repos are mentioned in the comment
```

---

### System Gap Analysis

System gap analysis compares what the architecture designed against what the workspace has implemented — across all repos and all contract boundaries. It builds on Layer 2's per-repo gap analysis (which identifies local gaps) by adding the system-level reference: capabilities described in architecture docs but absent from all repos, services that should exist but don't, and cross-repo inconsistencies in how the same concern is handled.

```gherkin
Feature: System Gap Analysis

  Scenario: Designed capability with no implementation is flagged
    Given the system architecture describes a rate-limiting layer at the API gateway
    And no rate-limiting implementation is found in the gateway repo or any service repo
    When the system gap analysis runs
    Then a gap item is created: type "designed-not-implemented", description "API rate limiting", severity high, source "architecture doc §4.2"

  Scenario: Duplicate implementations across services are identified
    Given payment-api, payment-data, and payment-reporting each implement their own JWT validation
    And the architecture mandates a shared auth service
    When the system gap analysis runs
    Then a gap item is flagged: type "duplication", affected-repos [payment-api, payment-data, payment-reporting], resolution "consolidate into shared auth service per architecture §3.1"

  Scenario: Missing service in the designed architecture is surfaced
    Given the architecture doc describes a notification service responsible for email and SMS
    And no notification service repo exists in the workspace
    When the system gap analysis runs
    Then a gap item is created: type "missing-service", description "notification service", source "architecture §6"
    And the gap appears as a dashed node in the service map

  Scenario: Cross-service inconsistency in shared concern is detected
    Given the architecture mandates all services use retry-with-backoff on external calls
    And only 3 of 7 services implement this pattern (detectable via Layer 1 call graph analysis)
    When the system gap analysis runs
    Then a gap item lists the 4 non-conformant services and their affected external call sites

  Scenario: System gap report is generated for planning
    Given a workspace with 7 repos and a system architecture doc
    When the developer runs sensei workspace analyse --gaps
    Then a prioritised report lists: designed-not-implemented capabilities, missing services, duplications, cross-service inconsistencies, and unguarded boundaries
    And each item includes: affected repos, source evidence, severity, and suggested resolution
    And gap items are stored in sensei.traceability and assignable to teams and sprints
```

---

### System Health Score

The system health score aggregates Layer 2 per-repo quality metrics (complexity, test coverage, doc coverage, conformance) with Layer 3 cross-repo metrics (open drift items, gap items, contract coverage, service map completeness) into a single workspace-level indicator. It is not a new measurement system — it is an aggregation of existing measurements from the layers below, adding only the cross-repo signals.

```gherkin
Feature: System Health Score

  Scenario: System health score aggregates layer metrics
    Given a workspace with 7 repos each having Layer 2 quality snapshots
    When the workspace index completes
    Then a system health snapshot is written with: average per-repo quality scores (50%), conformance score (20%), open drift items count (15%), open system gap items (15%)
    And the composite score is stored in sensei.quality_reports with type "workspace-health"

  Scenario: Health score trend reflects system improvement over time
    Given 90 days of workspace health snapshots
    When the team opens the Workspace Dashboard
    Then a trend chart shows the health score over time
    And drops are annotated with contributing events: "8 drift items opened during v2 API migration"
    And recoveries are annotated: "conformance score +18% after logging standardisation"

  Scenario: Per-team contribution to system health is visible
    Given 3 teams owning different services in the workspace
    When a team lead views their Team Health view
    Then they see their repos' contribution to the workspace score
    And they can compare their team's trend against the workspace average
    And other teams' repo contents are not exposed — only their quality score contributions

  Scenario: Health gate blocks CI when workspace health falls below threshold
    Given a CI workflow with sensei workspace health-check --min-score 70
    And the current workspace health score is 64 due to 5 open cross-repo drift items
    When the CI job runs
    Then it exits non-zero and prints: "Workspace health 64 is below threshold 70 — 5 open cross-repo drift items are the primary contributor"
    And it links to the workspace dashboard drift view
```
