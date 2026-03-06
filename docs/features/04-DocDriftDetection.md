# Doc Drift Detection

Code changes. Docs don't always follow. Over time, design docs describe a system that no longer exists, public docs reference APIs that have changed, and agents making decisions from stale docs produce incorrect results. Doc drift detection uses a traceability matrix and git history to flag exactly which docs are out of sync — no content analysis required.

## Features

### Git-Based Change Detection

Index runs store the current git commit hash. Subsequent drift checks use `git diff` against that commit to get the exact set of changed files — reliable across stash, pull, rebase, and checkout.

```gherkin
Feature: Git-Based Change Detection

  Scenario: Changed files detected via git diff
    Given a repo indexed at commit a3f8c21
    And three source files have been modified since that commit
    When the agent calls check_drift()
    Then only those three files are identified as changed
    And files modified before the last index are not flagged

  Scenario: Deleted files detected
    Given an indexed repo
    And src/legacy.ts has been deleted since the last index commit
    When the agent calls check_drift()
    Then src/legacy.ts is reported as deleted

  Scenario: Fallback to mtime/size for non-git repos
    Given a repo with no .git/ directory
    When the agent calls check_drift()
    Then changed files are detected via mtime and size comparison
    And the drift report notes git is not available
```

### Traceability Matrix

Each design/feature doc declares (or auto-detects) the source files it covers. When those source files change, the linked docs are flagged for review.

```gherkin
Feature: Traceability Matrix

  Scenario: Manual coverage declared in .llmspec.yaml
    Given docs/design/03-mcp-server.md declares it covers src/index.ts
    When the developer runs sensei index
    Then .index/traceability.json records that mapping

  Scenario: Auto-detection from doc content
    Given docs/design/07-drift.md mentions "src/tools/drift.ts" in its text
    And no manual declaration exists
    When the developer runs sensei index
    Then src/tools/drift.ts is auto-added to the traceability entry

  Scenario: Cross-reference drift: code changed, doc didn't
    Given docs/design/03-mcp-server.md covers src/index.ts
    And src/index.ts has changed since the last index
    And docs/design/03-mcp-server.md has NOT changed
    When the agent calls check_drift()
    Then docs/design/03-mcp-server.md is flagged as drifted
    And the report identifies src/index.ts as the trigger
```

### On-Demand Drift Reporting

Agents check drift at any time with a single tool call.

```gherkin
Feature: On-Demand Drift Reporting

  Scenario: No drift
    Given an indexed repo where no files have changed since the index commit
    When the agent calls check_drift()
    Then the response reports zero drifted docs
    And confirms all docs are aligned with code at the indexed commit

  Scenario: Actionable drift report
    Given three docs are drifted
    When the agent calls check_drift()
    Then each entry identifies the doc and which code file triggered the drift
    And the agent can address each item without additional lookups

  Scenario: Clean after resolving drift
    Given a repo with drifted docs that have been updated
    When the agent calls reindex_repo() after updating the docs
    Then check_drift() reports no drifted docs
```

### Pre-Commit Hook Integration

Drift is caught at commit time, before stale docs enter version control.

```gherkin
Feature: Pre-Commit Hook Integration

  Scenario: Hook blocks commit when docs are stale
    Given a pre-commit hook installed via sensei hooks install
    And a source file was modified without updating its design doc
    When the developer runs git commit
    Then the hook calls check_drift()
    And the commit is blocked with a report of which docs need updating

  Scenario: Hook passes when docs are in sync
    Given all doc layers are current
    When the developer runs git commit
    Then the hook reports no drift and the commit proceeds
```

### CI Integration

Drift detection in CI catches divergence before it reaches the main branch.

```gherkin
Feature: CI Integration

  Scenario: CI step reports drift as warning
    Given a CI pipeline with drift detection
    And a PR modifies code without updating public docs
    When the pipeline runs
    Then check_drift() produces a drift report saved as a CI artifact
    And the step is a warning (not failure) by default

  Scenario: CI step fails on drift when configured
    Given sensei drift --fail-on-drift is in the pipeline
    And drift is detected
    Then the CI step fails and the PR cannot merge until resolved
```

## Status

| Feature | Status |
|---------|--------|
| Git-based change detection (git diff vs lastIndexedCommit) | 🔲 Planned |
| Traceability matrix (.index/traceability.json) | 🔲 Planned |
| Manual coverage in .llmspec.yaml | 🔲 Planned |
| Auto-detection from doc content | 🔲 Planned |
| Cross-reference drift: code changed, linked doc didn't | 🔲 Planned |
| On-demand drift reporting (check_drift MCP tool) | 🔲 Planned |
| Pre-commit hook integration | 🔲 Planned |
| CI integration (--fail-on-drift) | 🔲 Planned |
| Non-git fallback (mtime/size) | 🔲 Planned |
