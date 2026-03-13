---
id: codebase-intelligence
type: feature
---

# Codebase Intelligence

> Sensei understands your repo so your agent doesn't have to explore it

Agents waste significant context window exploring unfamiliar repos — reading package.json, grepping for entry points, guessing project conventions. Sensei scans the repo once, extracts structured knowledge into Supabase, and delivers precise answers without requiring the agent to explore at all. Every subsequent session builds on the same persistent index, updated only where the code has changed. For existing codebases without documentation, sensei generates it from code. For teams whose docs live in Confluence, Notion, or a wiki, sensei connects to those systems directly. And for developers inheriting an unfamiliar or ageing codebase, sensei analyses its goals, quality, and gaps — providing the foundation for a confident refactor, enhancement, or rebuild.

## Features

### Repo Scanning

The scanner discovers all files, detects the project stack from manifest files (package.json, pyproject.toml, go.mod, Cargo.toml), extracts dev shortcuts from npm scripts, Makefile, and justfile, and produces a ScanResult with changedFiles and a gitDiff summary for downstream stages.

```gherkin
Feature: Repo Scanning

  Scenario: Scanner detects stack from manifest files
    Given a repo with package.json listing react and express
    And a pyproject.toml listing fastapi
    When the scanner runs
    Then the ScanResult stack includes typescript, react, express, and python
    And devDependencies like eslint and prettier are excluded from the stack

  Scenario: Scanner extracts dev shortcuts
    Given a repo with npm scripts dev, test, and build
    And a Makefile with targets migrate and seed
    When the scanner runs
    Then the ScanResult shortcuts include all npm scripts with their commands
    And the Makefile targets are listed alongside them

  Scenario: Scanner produces changedFiles and gitDiff
    Given a repo that has been previously indexed
    And two source files have been modified since the last scan
    When the scanner runs
    Then the ScanResult changedFiles lists only those two files
    And the ScanResult gitDiff contains the unified diff for those changes

  Scenario: Scanner ignores noise directories
    Given a repo containing node_modules/, dist/, .git/, and __pycache__/
    When the scanner runs
    Then none of those directories appear in the ScanResult file list
```

### Language-Agnostic AST Parsing

Sensei parses source files using language-specific AST parsers to extract structural information — symbols, their relationships, and their precise locations in code. Every major language used in the supported stacks has a dedicated parser. For languages without a dedicated parser, a best-effort fallback extracts what it can. Parser failures are isolated — one unparseable file never prevents the rest of the repo from being indexed.

```gherkin
Feature: Language-Agnostic AST Parsing

  Scenario: Source file is parsed into structured symbols
    Given a TypeScript source file with exported functions and classes
    When the parser runs on that file
    Then a structured result is produced containing symbol nodes with names, signatures, docstrings, and line ranges

  Scenario: Language without a dedicated parser falls back gracefully
    Given a source file in a language with no registered parser
    When the parse pipeline runs on that file
    Then a best-effort result is produced capturing whatever structure can be extracted
    And the pipeline continues without error

  Scenario: Custom parser handles domain-specific file types
    Given a repo containing contract files (.proto, .graphql) with no built-in parser
    And a custom parser configured for that file type
    When the parse pipeline runs on those files
    Then the custom parser is invoked and produces a structured result
    And the result is handled identically to any other parsed file

  Scenario: Parser failure is isolated and does not crash the pipeline
    Given one malformed file in a batch of 50
    When the parse pipeline processes the batch
    Then the malformed file is skipped with an error recorded in the index run summary
    And the remaining 49 files are parsed and indexed successfully
```

### Symbol Extraction

Sensei extracts functions, classes, and exports from every parsed file — capturing names, signatures, docstrings, and precise line ranges. The call graph records which symbols call which others. The dependency graph records which modules import which others. Together these three graphs — symbols, call relationships, and import relationships — are persisted for cross-session reuse and form the foundation for context ranking, system-level analysis, and quality measurement.

```gherkin
Feature: Symbol Extraction

  Scenario: Functions and classes are extracted with full metadata
    Given a source file with exported functions and inline documentation
    When symbol extraction runs on that file
    Then each function and class is captured with its name, signature, documentation, and line range
    And the distinction between exported and internal symbols is preserved

  Scenario: Call graph is built from symbol relationships
    Given a source file where function processOrder calls validateCart and chargePayment
    When symbol extraction runs
    Then call relationships are recorded: processOrder calls validateCart, processOrder calls chargePayment
    And these relationships are available for impact analysis and context ranking

  Scenario: Dependency graph is built from import relationships
    Given a module that imports AuthService from the auth module and db from the database module
    When symbol extraction runs
    Then both import relationships are recorded with their source, target, and imported names
    And the dependency graph reflects the actual module structure of the codebase

  Scenario: Symbol extraction output is available across sessions
    Given a repo that was indexed in a previous session
    When a new agent session begins
    Then all previously extracted symbols, call relationships, and dependency relationships are immediately available
    And no re-extraction is needed unless the underlying files have changed
```

### Semantic and Full-Text Search

Sensei supports three complementary search modes over the indexed codebase: semantic search (find by meaning, not keywords), keyword search (exact term matching across symbol names and descriptions), and relevance-ranked fallback when the primary strategies return sparse results. All three modes query the same persisted index — no re-parsing required.

```gherkin
Feature: Semantic and Full-Text Search

  Scenario: Agent finds code by semantic meaning
    Given an indexed repo with auth-related functions
    When the agent calls search_index("handle authentication errors")
    Then results include functions related to auth error handling
    And results are ranked by semantic similarity
    And no exact keyword match is required

  Scenario: Agent finds code by exact keyword
    Given an indexed repo
    When the agent calls search_index("JWT_SECRET", mode: "full-text")
    Then all symbols whose name or description contains JWT_SECRET are returned
    And each result includes file path, line number, and symbol name

  Scenario: BM25 fallback activates when graph is sparse
    Given an indexed repo where a semantic search for "retry backoff" returns 2 results
    When the search pipeline detects fewer than 3 semantic matches
    Then BM25 ranking is applied to the full symbol corpus
    And the combined result set is returned with source indicated per result

  Scenario: Combined search blends strategies by relevance score
    Given an indexed repo
    When the agent calls search_index("createUser")
    Then the pipeline checks symbol name match, then full-text, then semantic
    And results are merged and deduplicated
    And each result indicates which strategy produced the match
```

### Incremental Indexing

After the initial full scan, subsequent runs process only what has changed. Deleted files are removed from the index. The index always reflects the current state of the codebase — stale entries do not accumulate. A full rescan option is available when a clean rebuild is needed.

```gherkin
Feature: Incremental Indexing

  Scenario: Only changed files are re-processed
    Given a repo that has been indexed
    And 2 of 200 files have changed since the last index run
    When the developer runs sensei index
    Then only those 2 files are re-parsed and their symbols updated
    And the remaining 198 files are unchanged in the index
    And the run completes significantly faster than a full scan

  Scenario: Deleted files are removed from the index
    Given an indexed repo with src/legacy.ts in the index
    And src/legacy.ts has been deleted from the filesystem
    When the developer runs sensei index
    Then all index entries for src/legacy.ts are removed
    And no stale entries remain

  Scenario: Full rescan rebuilds the index from scratch
    Given an indexed repo
    When the developer requests a full rescan
    Then all files are re-parsed regardless of prior change detection
    And the entire index is rebuilt from the current state of the codebase

  Scenario: First run is always a full scan
    Given a repo with no prior index
    When the developer runs sensei index for the first time
    Then all files are scanned and parsed
    And the full index is written to the persistent store
```

### External Documentation Adapters

Not all teams keep docs in the repo. Confluence, Notion, GitHub Wiki, and SharePoint are common homes for design decisions, runbooks, and architecture notes. Sensei connects to these systems via doc system adapters — a parallel adapter layer to the language adapters. Fetched content is indexed alongside locally extracted docs, making external knowledge available to all context delivery, search, and traceability features.

```gherkin
Feature: External Documentation Adapters

  Scenario: Confluence space is indexed into doc_sections
    Given a repo with a Confluence space configured in .sensei/config.yaml
    When the developer runs sensei index
    Then the Confluence adapter fetches all pages in the configured space
    And each page is chunked and stored in sensei.doc_sections with source: "confluence"
    And embedded vectors are generated for semantic search

  Scenario: Notion workspace is indexed
    Given a Notion workspace URL and API token configured in .sensei/config.yaml
    When sensei index runs
    Then pages and databases in the workspace are fetched via the Notion API
    And their content is parsed and stored in sensei.doc_sections
    And page titles are stored as section headings for navigation

  Scenario: GitHub Wiki is indexed alongside repo code
    Given a repo with a GitHub Wiki enabled
    When sensei index runs
    Then wiki pages are fetched and stored in sensei.doc_sections
    And wiki content is available to find_doc and context_pack without additional configuration

  Scenario: External doc adapter fails gracefully
    Given a Confluence adapter configured with an expired API token
    When sensei index runs
    Then the Confluence adapter logs an auth error and is skipped
    And all other indexing (code, local docs) continues normally
    And the error is surfaced in the index run summary

  Scenario: Incremental sync updates only changed external pages
    Given a Confluence space already indexed
    And 3 of 40 pages have been updated since the last index
    When sensei index runs
    Then only those 3 pages are re-fetched and their doc_sections updated
    And the remaining 37 pages are unchanged
```

---

### Documentation Generation

For codebases with no existing documentation, sensei generates it from code. The local model reads extracted symbols, call graphs, and module structure to produce feature docs, design docs, API references, and onboarding guides. Generated docs are written to the repo's `docs/` directory (or a configured output path) and registered in `sensei.traceability` with `status: generated` — ready for human review and promotion to official docs.

```gherkin
Feature: Documentation Generation

  Scenario: Feature doc generated from a module with no existing doc
    Given a repo with an auth module containing 12 exported functions
    And no corresponding doc in docs/features/
    When the developer runs sensei doc generate --module src/auth
    Then a draft feature doc is generated in docs/features/auth.md
    And the doc includes: module purpose inferred from symbol names and docstrings, exported function list, usage examples derived from call graph consumers, and identified edge cases

  Scenario: Design doc generated for a complex module
    Given a module with deep call graph depth and multiple subsystems
    When the developer runs sensei doc generate --design src/pipeline
    Then a draft design doc is generated in docs/design/pipeline.md
    And it includes: component relationships from the call graph, data flow inferred from import chains, identified architectural patterns, and open questions flagged where intent is ambiguous

  Scenario: API reference generated from exported symbols
    Given a library package with exported TypeScript types and functions
    When the developer runs sensei doc generate --api packages/tools/src
    Then an API reference doc is generated with each export documented
    And parameter types, return types, and inferred descriptions are included
    And examples are derived from test files where available

  Scenario: Onboarding guide generated for developer orientation
    Given an indexed repo with no CONTRIBUTING.md or onboarding guide
    When the developer runs sensei doc generate --onboarding
    Then a getting-started guide is generated covering: project purpose, repo structure, how to run locally, key entry points, and common development workflows
    And the guide references actual shortcuts from sensei.project_profile

  Scenario: Generated docs are flagged for human review
    Given a newly generated feature doc
    When it is written to the docs/ directory
    Then the file includes a header comment: "Generated by sensei — review before publishing"
    And a traceability entry is created with status: "generated" awaiting promotion to "reviewed"
```

---

### Codebase Goal & Intent Analysis

When joining an existing project — or deciding whether to refactor, enhance, or rebuild — developers need to understand what a codebase is trying to do, how well it does it, and where the risks are. Sensei analyses the full index (symbols, call graphs, import chains, patterns, tests, docs) to produce a goal map and intent summary. This works across a single repo or a collection of related repos.

```gherkin
Feature: Codebase Goal and Intent Analysis

  Scenario: Goal map is generated for an unfamiliar repo
    Given a developer joins a project with no onboarding docs
    When they run sensei analyse --goals
    Then sensei produces a goal map summarising: primary purpose, major functional areas, key entry points, external integrations, and inferred user-facing capabilities
    And the summary is stored in sensei.project_profile under intent_summary
    And it is surfaced in get_session_context() for all future sessions

  Scenario: Multi-repo collection is analysed together
    Given three repos: an API, a dashboard, and a shared library
    And all three are registered in the same sensei workspace
    When the developer runs sensei analyse --goals --workspace
    Then sensei produces a cross-repo goal map showing: how the repos relate, which repo owns which capability, shared dependency flow, and the overall system purpose

  Scenario: Intent analysis surfaces undocumented architectural decisions
    Given a repo with no architecture docs
    And a consistent pattern of repository-layer separation inferred from symbol naming
    When sensei analyses the codebase intent
    Then it identifies the architectural pattern (e.g., layered architecture, hexagonal)
    And notes it as an inferred convention with supporting evidence (file paths, symbol names)
```

---

### Code Quality & Gap Analysis

Before a refactor, enhancement, or rebuild, developers need an objective view of code health. Sensei analyses the indexed codebase for quality signals: complexity hotspots, coupling, test coverage gaps, dead code, duplication, and pattern inconsistencies. The output is a prioritised gap report that guides where to start and what the riskiest areas are.

```gherkin
Feature: Code Quality and Gap Analysis

  Scenario: Complexity hotspots are identified
    Given an indexed repo
    When the developer runs sensei analyse --quality
    Then sensei identifies functions with high cyclomatic complexity (branching depth from call graph)
    And files with the most incoming call_edges (high coupling)
    And both are listed in the quality report ranked by severity

  Scenario: Test coverage gaps are surfaced
    Given a repo with test files indexed alongside source files
    When sensei analyses test coverage
    Then exported functions with no corresponding test file reference are listed as untested
    And modules with test-to-source ratios below a configurable threshold are flagged
    And the gap report includes the file path, function name, and call graph importance score

  Scenario: Dead code is detected
    Given an indexed repo with symbols in sensei.symbols
    When sensei analyses for unreachable symbols
    Then exported functions with zero call_edges inbound and no test references are flagged as potentially dead
    And the report distinguishes between definitely unused and possibly unused (e.g., dynamically called)

  Scenario: Pattern inconsistencies are reported
    Given a repo where 80% of route handlers follow an async/await pattern
    And 3 route handlers use raw Promise chains
    When sensei analyses pattern consistency
    Then the inconsistent handlers are listed with their file and line references
    And the dominant pattern is described so the developer can align the outliers

  Scenario: Refactor readiness report is generated
    Given a developer planning a major refactor
    When they run sensei analyse --refactor-readiness
    Then sensei produces a prioritised report: high-risk files (high complexity + low test coverage), coupling hotspots to decouple first, dead code safe to remove, and pattern inconsistencies to resolve before the refactor begins
    And each item includes an estimated effort indicator (low/medium/high) based on call graph depth

  Scenario: Multi-repo gap analysis identifies cross-repo inconsistencies
    Given two repos using the same shared library at different versions
    When sensei analyses the workspace
    Then the version mismatch is flagged
    And symbols available in the newer version but not used by the older consumer are listed as upgrade opportunities
```

---

### Three-Layer Metadata Model

Project knowledge is organized into three layers persisted in Supabase. Layer 1 is auto-extracted into `sensei.project_profile` (name, stack, entry points, shortcuts, detected patterns). Layer 2 is user-authored in `sensei.project_config` (description, guidelines, custom context — editable via the dashboard). Layer 3 consists of generated skills written to `.sensei/skills/`, produced by a local model from the combined Layer 1 and Layer 2 data.

```gherkin
Feature: Three-Layer Metadata Model

  Scenario: Layer 1 is auto-populated on index
    Given a repo with package.json, an src/index.ts entry point, and npm scripts
    When the indexer runs
    Then sensei.project_profile is upserted with name, stack, entry_points, and shortcuts
    And no user input is required

  Scenario: Layer 2 user edits are preserved across re-indexes
    Given a repo where the user has set a custom description and guidelines in project_config
    When the developer runs sensei index
    Then sensei.project_config rows are not overwritten
    And the user's description and guidelines remain intact

  Scenario: Layer 3 skills are regenerated when profile or config changes
    Given a repo with existing skills in .sensei/skills/
    And the user has updated their project_config description
    When the indexer detects the config change
    Then skills are regenerated from the updated Layer 1 + Layer 2 data
    And the new skill files are written to .sensei/skills/

  Scenario: All three layers are combined for agent orientation
    Given a fully indexed repo with all three layers populated
    When the agent calls get_session_context()
    Then the response includes auto-extracted profile data from Layer 1
    And user guidelines from Layer 2
    And skill references from Layer 3
```
