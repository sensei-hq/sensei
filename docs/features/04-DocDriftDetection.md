# Doc Drift Detection

Code changes. Docs don't always follow. Over time, design docs describe a system that no longer exists, public docs reference APIs that have changed, and agents making decisions from stale docs produce incorrect results. Doc drift detection tracks all three documentation layers together and reports divergence before it causes problems.

## Features

### Three-Layer Doc Tracking

Design docs, code, and public docs are treated as a system — not independent files. Changes to one layer without corresponding updates to others constitute drift.

```gherkin
Feature: Three-Layer Doc Tracking

  Scenario: Indexer fingerprints all doc layers
    Given a repo with docs/plans/, src/, and README.md
    When the agent calls reindex_repo()
    Then .index/doc-index.json records file paths, sizes, and modification times
    And all three layers are represented: design, code surface, and public docs

  Scenario: Fingerprints are updated on re-index
    Given a repo that has been indexed
    And two files have changed since the last index
    When the agent calls reindex_repo()
    Then the fingerprints for those two files are updated
    And fingerprints for unchanged files remain the same
```

### On-Demand Drift Reporting

Agents can check for drift at any time with a single tool call.

```gherkin
Feature: On-Demand Drift Reporting

  Scenario: No drift detected when files are unchanged
    Given an indexed repo where no files have changed
    When the agent calls check_drift()
    Then the response reports zero drifted files
    And confirms all indexed docs match current state

  Scenario: Modified file reported as drifted
    Given an indexed repo
    And src/auth.ts has been modified since the last index
    When the agent calls check_drift()
    Then src/auth.ts is listed as modified
    And the drift report notes the file has changed since last index

  Scenario: Deleted file reported as drifted
    Given an indexed repo
    And docs/plans/old-design.md has been deleted
    When the agent calls check_drift()
    Then docs/plans/old-design.md is listed as deleted
    And the agent is told the file was in the index but no longer exists

  Scenario: Drift report is actionable
    Given a drift report with three drifted files
    When the agent reviews the report
    Then each entry identifies the file and the nature of the drift
    And the agent can address each item without additional lookups

  Scenario: Clean state after resolving drift
    Given a repo with drifted files that have been updated
    When the agent calls reindex_repo() after updating the docs
    Then check_drift() reports no drifted files
```

### Pre-Commit Hook Integration

Drift can be caught at commit time, before stale docs enter version control.

```gherkin
Feature: Pre-Commit Hook Integration

  Scenario: Hook blocks commit when docs have drifted
    Given a repo with a pre-commit hook configured for drift checking
    And a source file has been modified without updating design docs
    When the developer runs git commit
    Then the hook calls check_drift()
    And the commit is blocked with a drift report
    And the developer is told which docs need updating

  Scenario: Hook passes when docs are in sync
    Given a repo with a pre-commit hook
    And all doc layers are current
    When the developer runs git commit
    Then the hook calls check_drift()
    And reports no drift
    And the commit proceeds
```

### CI Integration

Drift detection runs in CI to catch divergence before it reaches the main branch.

```gherkin
Feature: CI Integration

  Scenario: CI step reports drift as a warning
    Given a CI pipeline with drift detection configured
    And a PR that modifies code without updating public docs
    When the pipeline runs
    Then check_drift() produces a drift report
    And the report is saved as a CI artifact
    And the step is marked as a warning (not a failure, by default)

  Scenario: CI step fails on drift when configured
    Given a CI pipeline with --fail-on-drift configured
    And drift is detected
    When the pipeline runs
    Then the drift check step fails
    And the PR cannot merge until drift is resolved
```

## Status

| Feature | Status |
|---------|--------|
| Three-layer doc fingerprinting | 🔲 Planned |
| On-demand drift reporting (check_drift MCP tool) | 🔲 Planned |
| Modified file detection | 🔲 Planned |
| Deleted file detection | 🔲 Planned |
| New file detection (unindexed docs) | 🔲 Planned |
| Pre-commit hook integration | 🔲 Planned |
| CI integration | 🔲 Planned |
