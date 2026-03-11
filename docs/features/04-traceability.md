---
id: traceability
type: feature
---

# Traceability

Code changes. Docs don't always follow. The traceability module records which source files each doc covers, uses git history to detect when covered files change, and flags exactly which docs are out of sync — no content analysis required. Drift detection is a pure structural operation against the traceability matrix.

## Features

### Declare Coverage in LLMSpec

Developers declare which code files each doc covers directly in `.llmspec.yaml`. This is the authoritative source.

```gherkin
Feature: Manual Coverage Declaration

  Scenario: Developer declares doc-to-code coverage
    Given docs/design/03-mcp-server.md describes the MCP server
    And the developer adds it to .llmspec.yaml under docs[].covers[]
    When the developer runs sensei index
    Then .index/traceability.json records the mapping
    And check_drift() uses that mapping for cross-reference detection

  Scenario: Coverage persists across reindex
    Given manual coverage declared in .llmspec.yaml
    When the developer runs sensei index again
    Then the manual entries are preserved in traceability.json
    And auto-detected entries are merged but do not overwrite manual ones
```

### Auto-Detection from Doc Content

For docs without manual declarations, `sensei index` scans doc content for filename references and symbol mentions and builds inferred entries.

```gherkin
Feature: Auto-Detection

  Scenario: Doc mentions a source file path
    Given docs/design/07-drift.md contains the text "src/tools/drift.ts"
    And no manual declaration exists for that doc
    When the developer runs sensei index
    Then src/tools/drift.ts is auto-added to docs/design/07-drift.md's coverage entry

  Scenario: Symbol reference creates inferred link
    Given docs/design/03-mcp-server.md mentions the function "checkDrift"
    And checkDrift is exported from src/tools/drift.ts per the symbol-map
    When the developer runs sensei index
    Then src/tools/drift.ts is inferred as covered by docs/design/03-mcp-server.md

  Scenario: Manual entry not overwritten by auto-detection
    Given a manual entry declares docs/design/03-mcp-server.md covers src/index.ts
    And auto-detection would also find src/tools/query.ts
    When sensei index runs
    Then both entries are in traceability.json
    And the manual entry is unchanged
```

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

### Precise Drift Cross-Reference

When drift is checked, the traceability matrix is used to find docs that need attention based on what actually changed in git.

```gherkin
Feature: Cross-Reference Drift

  Scenario: Code changed, linked doc didn't
    Given docs/design/03-mcp-server.md covers src/index.ts
    And src/index.ts changed since the last index commit
    And docs/design/03-mcp-server.md did not change
    When the developer runs sensei drift
    Then docs/design/03-mcp-server.md is flagged as drifted
    And the report shows src/index.ts as the trigger file

  Scenario: Unlinked doc not falsely flagged
    Given docs/plans/2026-03-01-spike.md has no coverage entries
    And many source files changed
    When the developer runs sensei drift
    Then docs/plans/2026-03-01-spike.md is NOT in the drift report

  Scenario: Doc updated with its code
    Given docs/design/07-drift.md covers src/tools/drift.ts
    And both files changed since the last index
    When the developer runs sensei drift
    Then docs/design/07-drift.md is NOT flagged as drifted
    And the co-change is considered aligned
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
| Manual coverage declaration in .llmspec.yaml | 🔲 Planned |
| .index/traceability.json generation on reindex | 🔲 Planned |
| Auto-detection from filename mentions in docs | 🔲 Planned |
| Auto-detection from symbol references | 🔲 Planned |
| Manual entries not overwritten by auto-detection | 🔲 Planned |
| Cross-reference drift: code changed, linked doc didn't | 🔲 Planned |
| Co-change (code + doc both changed) treated as aligned | 🔲 Planned |
| Git-based change detection (git diff vs lastIndexedCommit) | 🔲 Planned |
| Non-git fallback (mtime/size) | 🔲 Planned |
| On-demand drift reporting (check_drift MCP tool) | 🔲 Planned |
| Pre-commit hook integration | 🔲 Planned |
| CI integration (--fail-on-drift) | 🔲 Planned |
