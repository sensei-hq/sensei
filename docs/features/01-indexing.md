---
id: indexing
type: feature
---

# Indexing

The indexer is the foundation of sensei. It scans a repo once to produce structured orientation artifacts — LLMSpec, symbol map, llms.txt, multi-modal search index, and symbol graph — that let any agent orient in a single tool call. After the initial scan, subsequent runs are incremental: only changed files are re-processed, keeping the index fast and current regardless of repo size.

## Features

### Repo Scanner

The indexer scans a repo and extracts everything an agent needs to start working: file structure, tech stack, dev commands, exported symbols, code patterns, and all documentation layers.

```gherkin
Feature: Repo Scanner

  Scenario: Agent indexes an unfamiliar repo
    Given a code repository with source files, package.json, and docs
    When the agent calls reindex_repo()
    Then a .index/ directory is created with structured artifacts
    And a .llmspec.yaml is created in .index/
    And a CLAUDE.md is created at the repo root if one does not exist
    And a llms.txt is created in .index/

  Scenario: Indexer extracts tech stack from package.json
    Given a repo with package.json listing react and express
    When the agent calls reindex_repo()
    Then .index/stack.md lists typescript/javascript, react, and express
    And does not include dev tooling like eslint or prettier

  Scenario: Indexer extracts dev shortcuts
    Given a repo with package.json scripts: dev, test, build
    When the agent calls reindex_repo()
    Then .index/shortcuts.md lists each script with its command

  Scenario: Indexer identifies entry points
    Given a repo with src/index.ts and src/router.ts
    When the agent calls reindex_repo()
    Then the .llmspec.yaml entry_points lists these files with inferred roles

  Scenario: Indexer skips noise
    Given a repo with node_modules/, dist/, and .git/ directories
    When the agent calls reindex_repo()
    Then none of these directories are included in the index
```

### LLMSpec Generation

The `.llmspec.yaml` is an OpenAPI-equivalent for LLM consumption. It is the primary orientation artifact — structured, compact, and queryable.

```gherkin
Feature: LLMSpec Generation

  Scenario: New LLMSpec is created on first index
    Given a repo with no existing .llmspec.yaml
    When the agent calls reindex_repo()
    Then a .llmspec.yaml is created with auto-populated fields
    And fields that require human judgment (concepts, patterns) are marked TODO

  Scenario: Existing LLMSpec is preserved on re-index
    Given a repo with a manually edited .llmspec.yaml
    When the agent calls reindex_repo()
    Then the existing .llmspec.yaml is not overwritten
    And the agent is told to review .index/ for updated data

  Scenario: Agent retrieves a section of the LLMSpec
    Given an indexed repo with a populated .llmspec.yaml
    When the agent calls get_llmspec("entry_points")
    Then only the entry_points section is returned
    And the full spec is not loaded into context

  Scenario: LLMSpec enables 500-token orientation
    Given a populated .llmspec.yaml for a large codebase
    When the agent calls get_llmspec()
    Then the agent receives project name, description, stack, entry points,
         key concepts, core patterns, and dev shortcuts
    And the total output is under 600 tokens for a typical project
```

### Symbol Map

Exported symbols are stored at four resolution levels so agents can discover what exists cheaply and only load source when they need to edit.

```gherkin
Feature: Symbol Map

  Scenario: Exported functions are captured at L0
    Given a TypeScript file with three exported functions
    When the agent calls reindex_repo()
    Then .index/symbol-map.json contains L0 signatures for each function
    And each signature includes parameter types and return type

  Scenario: Agent retrieves module exports at L0
    Given an indexed repo
    When the agent calls list_exports("src/auth")
    Then a list of function signatures is returned
    And no function implementations are included
    And token usage is under 100 tokens for a typical module

  Scenario: Agent retrieves a file at full source
    Given an indexed repo
    When the agent calls get_file_context("src/auth.ts", "L3")
    Then the full source of src/auth.ts is returned

  Scenario: Docstrings are excluded from all levels
    Given a TypeScript file where functions have JSDoc comments
    When the agent retrieves the file at any level below L3
    Then docstrings are not included in the response
    And the signature and logic are still present
```

### llms.txt Generation

The `llms.txt` file follows the [llmstxt.org](https://llmstxt.org) standard — a concise, plain-text project summary designed for LLM consumption.

```gherkin
Feature: llms.txt Generation

  Scenario: llms.txt is generated on index
    Given a repo with a README.md
    When the agent calls reindex_repo()
    Then a llms.txt is created summarising the project
    And the content follows the llmstxt.org format

  Scenario: Agent can request llms.txt regeneration
    Given an indexed repo where the README has been updated
    When the agent calls generate_llms_txt()
    Then a fresh llms.txt is written reflecting current content
```

### Multi-Modal Search

The index supports semantic, full-text, and symbol search so agents can find relevant code using whichever strategy fits the query.

```gherkin
Feature: Multi-Modal Search

  Scenario: Agent finds code by semantic meaning
    Given an indexed repo
    When the agent calls search_index("handle authentication errors")
    Then results include files and functions related to auth error handling
    And results are ranked by semantic relevance
    And no exact keyword match is required

  Scenario: Agent finds code by exact text
    Given an indexed repo
    When the agent calls search_index("JWT_SECRET", mode: "full-text")
    Then all occurrences of "JWT_SECRET" across source and config files are returned
    And line numbers and file paths are included

  Scenario: Agent finds code by symbol name
    Given an indexed repo with a symbol map
    When the agent calls search_index("validateToken", mode: "symbol")
    Then all exported and local symbols matching "validateToken" are returned
    And each result includes: file path, line number, and signature

  Scenario: Agent uses multi-modal search without specifying mode
    Given an indexed repo
    When the agent calls search_index("createUser")
    Then the index runs symbol lookup first, falls back to full-text, then semantic
    And returns the best matches across all three strategies
    And the response indicates which strategy matched

  Scenario: Search results are token-efficient
    Given a large repo with 500 files
    When the agent calls search_index("database connection pool")
    Then at most 10 results are returned
    And each result is a file path + one-line summary, not full content
    And total output is under 300 tokens
```

### Symbol Graph

Related symbols are linked in a graph so agents can trace call chains, find dependents, and understand scope of impact before making changes.

```gherkin
Feature: Symbol Graph

  Scenario: Indexer builds symbol relationships
    Given a repo where function A calls function B and function C
    When the agent calls reindex_repo()
    Then .index/symbol-graph.json contains edges: A→B and A→C
    And each edge records the file, line, and relationship type (call, import, extend)

  Scenario: Agent finds all callers of a function
    Given an indexed symbol graph
    When the agent calls get_callers("validateToken")
    Then all functions that call validateToken are returned
    And each result includes: caller name, file path, and line number
    And the response is under 200 tokens for a typical function

  Scenario: Agent finds all symbols a function depends on
    Given an indexed symbol graph
    When the agent calls get_dependencies("processRequest")
    Then all symbols called or imported by processRequest are returned
    And transitive dependencies beyond one hop are not included unless requested

  Scenario: Agent assesses impact of changing a symbol
    Given an indexed symbol graph
    When the agent calls get_callers("parseConfig", depth: 2)
    Then direct and indirect callers up to 2 hops are returned
    And the agent can estimate the blast radius before modifying the symbol

  Scenario: Symbol graph is updated incrementally
    Given an indexed symbol graph
    And one source file has changed
    When the agent calls reindex_repo()
    Then only the edges involving symbols in the changed file are updated
    And unchanged edges are preserved
```

### Incremental Indexing

After the initial full scan, subsequent runs only process changed files.

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
| Repo scanner (file map, stack, shortcuts, symbols) | 🔲 Planned |
| LLMSpec (.llmspec.yaml) generation | 🔲 Planned |
| CLAUDE.md generation | 🔲 Planned |
| llms.txt generation | 🔲 Planned |
| Symbol map at L0–L2 | 🔲 Planned |
| Doc layer fingerprinting | 🔲 Planned |
| Project-scoped skill generation | 🔲 Planned |
| Multi-modal search (semantic + full-text + symbol) | 🔲 Planned |
| Symbol graph (callers, dependencies, impact analysis) | 🔲 Planned |
| Full scan on first run | ✅ Done |
| Incremental scan on subsequent runs | 🔲 Planned |
| Deleted file removal from index | 🔲 Planned |
| --force flag for full rescan | 🔲 Planned |
| Index summary output | 🔲 Planned |
