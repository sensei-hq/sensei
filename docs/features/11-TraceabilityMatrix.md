# Traceability Matrix

Design docs, feature docs, and READMEs each describe specific parts of the codebase. The traceability matrix records those relationships explicitly — which source files each doc covers. Combined with git history, this enables precise drift detection without content analysis: when a source file changes, only the docs that cover it are flagged.

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

For docs that don't have manual declarations, `sensei index` scans doc content for filename references and symbol mentions and builds inferred entries.

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
