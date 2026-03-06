# Incremental Indexing

Running a full repo scan on every `sensei index` invocation is wasteful on large repos. After the first index, most files haven't changed. Incremental indexing detects which files have been added, modified, or deleted since the last run and only re-processes those — making subsequent index runs fast regardless of repo size.

## Features

### Incremental Index on Subsequent Runs

After an initial full index, subsequent runs only process changed files.

```gherkin
Feature: Incremental Index

  Scenario: Only changed files are re-processed
    Given a repo that has been indexed
    And only 2 files have changed since the last index
    When the developer runs sensei index
    Then only those 2 files are re-extracted
    And the rest of the symbol-map is unchanged
    And the run completes significantly faster than a full scan

  Scenario: New files are added to the index
    Given an indexed repo
    And a new file src/payments/invoice.ts has been created
    When the developer runs sensei index
    Then the new file is added to the symbol-map
    And existing entries are unchanged

  Scenario: Deleted files are removed from the index
    Given an indexed repo with src/legacy.ts in the symbol-map
    And src/legacy.ts has been deleted
    When the developer runs sensei index
    Then src/legacy.ts is removed from the symbol-map
    And no stale entries remain

  Scenario: First run is always a full scan
    Given a repo with no .index/ directory
    When the developer runs sensei index
    Then all files are scanned and indexed
    And .index/ is created with all artifacts
```

### Force Full Reindex

Escape hatch for when a full rescan is needed.

```gherkin
Feature: Force Full Reindex

  Scenario: Developer forces full reindex
    Given an indexed repo
    When the developer runs sensei index --force
    Then all files are re-scanned regardless of fingerprints
    And the symbol-map is rebuilt from scratch
    And doc-index fingerprints are refreshed

  Scenario: Force flag documented in help
    When the developer runs sensei index --help
    Then the output explains: --force runs a full scan regardless of prior index
```

### Index Summary Output

After indexing, the developer sees what changed.

```gherkin
Feature: Index Summary

  Scenario: Summary shown after incremental run
    Given a repo where 3 files changed and 1 was deleted
    When sensei index completes
    Then the output shows:
      Indexed: 3 files updated, 1 removed, 42 unchanged
      Artifacts: .index/ updated

  Scenario: Summary shown after first run
    Given a repo with no prior index
    When sensei index completes
    Then the output shows:
      Indexed: 45 files (full scan)
      Created: .llmspec.yaml, CLAUDE.md, llms.txt, .index/
```

## Status

| Feature | Status |
|---------|--------|
| Full scan on first run | ✅ Done |
| Incremental scan on subsequent runs | 🔲 Planned |
| Deleted file removal from index | 🔲 Planned |
| --force flag for full rescan | 🔲 Planned |
| Index summary output | 🔲 Planned |
