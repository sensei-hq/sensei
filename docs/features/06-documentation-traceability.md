---
id: documentation-traceability
type: feature
---

# Documentation Traceability

> Sensei keeps design docs, code, and specs in sync

Design decisions made in documents and the code that implements them drift apart silently — a renamed function, a removed endpoint, a changed data model — until a future agent or developer discovers the gap at the worst possible moment. Sensei closes this loop by maintaining a live traceability graph in Supabase that links feature docs, code files, and test results, detects drift automatically on every git operation, and gives developers tools to query, scaffold, and export coverage without leaving their workflow. For codebases with missing documentation, traceability entries are auto-created from generated docs and from code quality gap analysis — making gaps first-class trackable work items that can be assigned to sprints, resolved, and verified automatically on the next index run.

## Features

### Doc-to-Code Coverage

Traceability records are stored in `sensei.traceability` (Supabase) with fields for `feature_id`, `item_id`, `status`, `design_doc`, `code_file`, `sprint`, and `cycle`. `traceability.yaml` is a generated export used by CI and offline tooling — it is not the source of truth. The dashboard reads from the database and renders feature docs as markdown with Mermaid diagrams.

```gherkin
Feature: Doc-to-Code Coverage

  Scenario: Traceability record is created when a feature doc is linked to a code file
    Given a feature doc docs/features/05-library-intelligence.md
    And a code file packages/collector/src/lib-indexer.ts that implements it
    When the developer runs sensei traceability link 05-library-intelligence packages/collector/src/lib-indexer.ts
    Then a row is inserted into sensei.traceability with the feature_id and code_file
    And the status is set to "in-progress"

  Scenario: traceability.yaml is generated as a CI export
    Given the sensei.traceability table has 12 records for the current repo
    When the developer runs sensei traceability export
    Then a traceability.yaml file is written to the repo root
    And it contains all 12 records serialised as YAML
    And a header comment notes it is generated and the source of truth is Supabase

  Scenario: Dashboard renders feature coverage with Mermaid
    Given sensei.traceability has records linking three feature docs to code files
    When the developer opens the traceability dashboard view
    Then each feature doc is shown with its linked code files and current status
    And a Mermaid diagram shows the coverage graph for the selected sprint
```

### Drift Detection

Drift occurs when a code file changes but its linked design document has not been updated within the same git operation. Sensei cross-references `git diff` output against `sensei.traceability` and flags mismatched pairs. The `check_drift` MCP tool returns the current drift list; a pre-commit hook and CI flag enforce it at commit and pipeline boundaries.

```gherkin
Feature: Drift Detection

  Scenario: Drift is flagged when code changes without a doc update
    Given packages/collector/src/lib-indexer.ts is linked to 05-library-intelligence.md in sensei.traceability
    When a developer commits a change to lib-indexer.ts without touching 05-library-intelligence.md
    Then the pre-commit hook detects drift
    And reports: "lib-indexer.ts changed but linked doc 05-library-intelligence.md was not updated"
    And blocks the commit (when mode is "block")

  Scenario: check_drift MCP tool returns drifted pairs
    Given two code files have changed since their linked docs were last updated
    When the agent calls check_drift()
    Then a list of two drifted pairs is returned
    And each pair includes: code_file, design_doc, last_code_change, last_doc_change

  Scenario: CI pipeline fails on drift when flag is set
    Given the CI workflow runs sensei check-drift --fail-on-drift
    And there is one drifted pair in sensei.traceability
    When the CI job executes
    Then the job exits with a non-zero status code
    And the drifted pair is printed to stdout for the developer to resolve

  Scenario: Drift check passes when doc is updated alongside code
    Given lib-indexer.ts is linked to 05-library-intelligence.md
    When a developer commits changes to both files in the same commit
    Then the pre-commit hook finds no drift
    And the commit proceeds without interruption
```

### Doc Doctor

`sensei doctor` inspects documentation files against a configurable rule set and reports issues with file-and-line references. Rules are defined in `.sensei/config.yaml` or inherited from `project_config`. A single file or an entire directory can be checked.

```gherkin
Feature: Doc Doctor

  Scenario: Single file is checked for broken links
    Given docs/features/05-library-intelligence.md contains a link to a file that no longer exists
    When the developer runs sensei doctor docs/features/05-library-intelligence.md
    Then the output reports: "05-library-intelligence.md:34 — broken link: ../design/lib-cache.md not found"
    And exits with a non-zero code

  Scenario: Directory batch check reports issues across all docs
    Given docs/features/ contains 8 files, two of which have missing required sections
    When the developer runs sensei doctor docs/features/
    Then both files are reported with their missing sections and line references
    And files with no issues are listed as passing

  Scenario: Doc with missing required section is flagged
    Given a feature doc that is missing the "## Features" section required by the feature template
    When the developer runs sensei doctor on that file
    Then the output reports: "<file>:1 — missing required section: ## Features"
```

### Feature Scaffold

`sensei doc new <feature-name>` generates a pre-populated feature document from the appropriate template, creates a corresponding `sensei.traceability` entry with `status: planned`, and links to any referenced design docs detected in the project.

```gherkin
Feature: Feature Scaffold

  Scenario: New feature doc is generated from the feature template
    Given the developer runs sensei doc new token-budget
    When the command completes
    Then docs/features/token-budget.md is created using the feature doc template
    And the frontmatter id is set to "token-budget"
    And section stubs (Goal, Features) are pre-populated with placeholder text

  Scenario: Traceability entry is created with status planned
    Given the developer runs sensei doc new token-budget
    When the command completes
    Then a row is inserted into sensei.traceability with feature_id "token-budget" and status "planned"

  Scenario: Template is auto-detected from output path
    Given the developer runs sensei doc new arch/pipeline-design --output docs/design/
    When the command completes
    Then the design doc template is used instead of the feature template
    And docs/design/pipeline-design.md is created with design-appropriate section stubs
```

### find_doc MCP Tool

`find_doc` is a documentation-first MCP tool that accepts a topic or symbol name and returns matching sections from `sensei.doc_sections` using pgvector semantic search. It is distinct from `search`, which is code-first.

```gherkin
Feature: find_doc MCP Tool

  Scenario: Agent retrieves doc sections by topic
    Given docs/features/05-library-intelligence.md is indexed in sensei.doc_sections
    When the agent calls find_doc("library indexing pipeline")
    Then the matching sections from 05-library-intelligence.md are returned
    And each result includes: doc_path, section_heading, content_excerpt, and similarity_score

  Scenario: Agent retrieves doc sections by symbol name
    Given sensei.doc_sections contains references to the symbol "context_pack"
    When the agent calls find_doc("context_pack")
    Then all doc sections mentioning context_pack are returned
    And results are ranked by semantic similarity, not keyword frequency

  Scenario: find_doc returns docs, not source files
    Given the project has both a feature doc and source code related to "token budget"
    When the agent calls find_doc("token budget")
    Then only documentation sections are returned
    And no source file paths appear in the results
```

### Traceability from Generated Docs & Gap Analysis

When sensei generates documentation from code, traceability entries are created automatically — linking the generated doc to its source module with `status: generated`. When code quality analysis identifies gaps (untested functions, undocumented modules, dead code, pattern inconsistencies), each gap becomes a traceability item with `status: gap`. Both types are first-class work items: visible in the dashboard, assignable to sprints, and auto-resolved when the underlying condition is fixed on the next index run.

```gherkin
Feature: Traceability from Generated Docs and Gap Analysis

  Scenario: Generated doc creates a traceability entry
    Given the developer runs sensei doc generate --module src/payments
    And a doc is generated at docs/features/payments.md
    When the generation completes
    Then a row is inserted into sensei.traceability with feature_id "payments", code_file "src/payments", and status "generated"
    And the entry appears in the dashboard under "Needs Review"

  Scenario: Generated doc is promoted to reviewed
    Given a traceability entry with status "generated" for payments.md
    When the developer reviews the doc, makes edits, and runs sensei traceability promote payments
    Then the status is updated to "reviewed"
    And the "Generated by sensei" header comment is removed from the doc file

  Scenario: Untested function creates a gap traceability item
    Given sensei quality analysis identifies src/auth/token.ts:validateToken as having no test coverage
    When the analysis run completes
    Then a traceability item is created: feature_id "quality-gap", code_file "src/auth/token.ts", item "test-coverage:validateToken", status "gap"
    And it appears in the dashboard under "Quality Gaps"

  Scenario: Gap is auto-resolved when fixed
    Given a traceability item with status "gap" for test-coverage:validateToken
    And the developer adds a test for validateToken in tests/auth/token.spec.ts
    When the developer runs sensei index
    Then the gap item's status is automatically updated to "resolved"
    And the resolution timestamp and resolving commit hash are recorded

  Scenario: All gap types are surfaced in a unified gap report
    Given a repo with 3 untested functions, 2 undocumented modules, 1 dead code symbol, and 4 pattern inconsistencies
    When the developer runs sensei analyse --gaps
    Then a unified gap report is printed showing all 10 items grouped by type
    And each item shows: file, line, severity, and estimated effort
    And the total is: 3 test gaps, 2 doc gaps, 1 dead code item, 4 consistency issues
```

---

### Quality Metrics & Reports

Code quality metrics are stored in `sensei.quality_reports` as time-series snapshots — not one-off CLI output. Each index run appends a new quality snapshot, enabling trend analysis: is complexity decreasing? Is test coverage improving? Is dead code being removed? Reports are queryable from CLI and visual in the dashboard.

```gherkin
Feature: Quality Metrics and Reports

  Scenario: Quality snapshot is recorded on every index run
    Given a repo being actively developed
    When the developer runs sensei index
    Then a quality_reports row is inserted with: timestamp, complexity_score, test_coverage_pct, dead_code_count, doc_coverage_pct, and pattern_consistency_score

  Scenario: Quality trend is visible in the dashboard
    Given 60 days of quality snapshots in sensei.quality_reports
    When the developer opens the Code Quality view
    Then trend lines are shown for each metric over time
    And weeks with major refactors are visible as step-changes in the trend

  Scenario: Quality report is exported from CLI
    Given quality data for the current repo in sensei.quality_reports
    When the developer runs sensei quality report
    Then a formatted report is printed showing: current scores, 30-day deltas, top 5 complexity hotspots, test coverage gaps by module, and dead code candidates
    And the report can be output as JSON with --json for CI integration

  Scenario: Quality gate blocks CI when thresholds are breached
    Given a CI workflow configured with sensei quality check --min-coverage 80
    And the current test_coverage_pct is 74
    When the CI job runs
    Then it exits with a non-zero status code
    And prints: "Test coverage 74% is below the required 80% threshold"

  Scenario: Multi-repo quality comparison is available in workspace view
    Given three repos in a sensei workspace each with quality snapshots
    When the developer opens the Workspace Quality view
    Then a table shows each repo's current quality scores side by side
    And repos below threshold on any metric are highlighted
```

---

### Sprint and Cycle Planning

Traceability items carry `sprint` and `cycle` fields in Supabase, enabling planning and progress tracking without leaving sensei. The dashboard supports drag-to-sprint assignment, and `sensei traceability export` produces a filtered snapshot for the current sprint.

```gherkin
Feature: Sprint and Cycle Planning

  Scenario: Traceability items are assigned to a sprint in the dashboard
    Given three traceability items with status "planned" and no sprint assigned
    When the developer drags them onto "Sprint 4" in the dashboard
    Then each item's sprint field is updated to "sprint-4" in sensei.traceability

  Scenario: Sprint snapshot is exported to YAML
    Given sprint-4 has six traceability items across two features
    When the developer runs sensei traceability export --sprint sprint-4
    Then traceability.yaml contains only the six sprint-4 items
    And the file header identifies the sprint and export timestamp

  Scenario: Items are filtered by status in the dashboard
    Given sprint-4 has items with statuses: planned, in-progress, and done
    When the developer selects the "in-progress" filter in the dashboard
    Then only in-progress items are shown
    And the item count reflects the filtered subset
```
