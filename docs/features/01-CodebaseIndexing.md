# Codebase Indexing

An AI agent dropped into a large codebase without orientation is expensive — it reads files, greps broadly, loads directories, and still might miss key patterns. Codebase indexing solves this by scanning a repo once and producing structured artifacts that let any agent orient in a single tool call.

## Features

### Repo Scanner

The indexer scans a repo and extracts everything an agent needs to start working: file structure, tech stack, dev commands, exported symbols, code patterns, and all documentation layers.

```gherkin
Feature: Repo Scanner

  Scenario: Agent indexes an unfamiliar repo
    Given a code repository with source files, package.json, and docs
    When the agent calls reindex_repo()
    Then a .index/ directory is created with structured artifacts
    And a .llmspec.yaml is created at the repo root
    And a CLAUDE.md is created at the repo root if one does not exist
    And a llms.txt is created at the repo root

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

### Incremental Re-indexing

Re-running the indexer only processes changed files, keeping re-indexing fast.

```gherkin
Feature: Incremental Re-indexing

  Scenario: Re-index after a small change
    Given a repo that has been indexed previously
    And one source file has changed since the last index
    When the agent calls reindex_repo()
    Then only the changed file's symbols are updated
    And unchanged files are not re-processed

  Scenario: Full re-index is available
    Given an indexed repo
    When the agent calls reindex_repo({ force: true })
    Then all files are re-processed regardless of change state
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
| Incremental re-indexing | 🔲 Planned |
| Project-scoped skill generation | 🔲 Planned |
