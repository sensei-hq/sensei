---
id: library-intelligence
type: feature
---

# Library Intelligence

> Sensei knows about the libraries you use — internal and external

Third-party libraries and internal shared packages are a major source of agent confusion: the model either hallucinates an API it doesn't know or spends tokens crawling docs it has already seen. Sensei solves this by building a two-tier cache — root index for fast lookup, on-demand page cache for deep retrieval — and by generating concise skill files from custom library source so agents work with local knowledge rather than generic training data. Every library is a first-class citizen: indexing, ranking, and context assembly treat custom libs the same as project code.

## Features

### Custom Library Indexing

Internal shared libraries (e.g. rokkit, kavach, dbd) are indexed from source using the same pipeline as project code. The source path is declared in `.sensei/config.yaml` under `custom_libs`, and each library is tracked separately so ranking and slicing queries treat them consistently with project code.

```gherkin
Feature: Custom Library Indexing

  Scenario: Developer registers a custom library
    Given a .sensei/config.yaml with custom_libs entry for "rokkit" at "../rokkit/src"
    When the developer runs sensei index
    Then rokkit source files are scanned and parsed
    And rokkit symbols are indexed and appear in search results alongside project code

  Scenario: Incremental re-index on file change
    Given rokkit has been indexed and one file "rokkit/src/button.ts" has changed
    When the developer runs sensei index
    Then only the changed file is re-parsed
    And unchanged rokkit files are not re-processed
    And the symbol index for that file is updated

  Scenario: Force full rebuild of a custom library
    Given rokkit has been indexed
    When the developer requests a full rescan
    Then all rokkit source files are re-parsed regardless of prior change detection
    And the complete symbol index for rokkit is rebuilt from scratch

  Scenario: Custom lib symbols are included in context_pack
    Given rokkit is indexed and a project file imports ButtonGroup from rokkit
    When the agent calls context_pack for a task touching that import
    Then rokkit/src/button-group.ts symbols appear in the packed context
    And the response identifies them as library symbols
```

### llms.txt Priority

`llms.txt` is the highest-priority documentation source for any library. When present — either as a bundled file path or a remote URL — sensei parses it first and supplements with source indexing only for symbols not covered by the llms.txt content.

```gherkin
Feature: llms.txt Priority

  Scenario: Remote llms.txt is fetched and parsed first
    Given a custom_libs entry for "rokkit" with a configured llms.txt URL
    When the developer runs sensei index
    Then sensei fetches the llms.txt content before scanning source
    And documentation from llms.txt is indexed with priority
    And source indexing only runs for symbols absent from the llms.txt content

  Scenario: Bundled llms.txt file is used when a URL is not set
    Given a custom_libs entry for "kavach" with a local llms.txt path configured
    When the developer runs sensei index
    Then sensei reads the local llms.txt file
    And its content is parsed and indexed before source files are scanned

  Scenario: llms.txt chunks take precedence in search results
    Given kavach is indexed with both llms.txt content and source symbols
    When the agent calls get_lib_docs("kavach", "middleware")
    Then chunks sourced from llms.txt are ranked above source-derived chunks
    And the response indicates each chunk's source as "llms_txt" or "source"
```

### External Doc Registry

The bundled library registry maps known libraries to their documentation URL templates and section structure. The registry is updated via `sensei update-registry` and accepts community pull requests. Per-repo library usage is tracked and doc content is cached globally, shared across all repos.

```gherkin
Feature: External Doc Registry

  Scenario: Registry entry drives doc URL resolution
    Given the library registry contains an entry for "zod" with a documentation URL template
    And the project's package.json lists "zod" as a dependency
    When the agent calls get_lib_docs("zod", "schema")
    Then sensei constructs the URL from the template and fetches the relevant page
    And the fetched content is cached globally for reuse

  Scenario: Detected import registers library for doc retrieval
    Given the project imports from "drizzle-orm" in three source files
    When sensei index runs
    Then "drizzle-orm" is registered as a library used by this repo
    And its documentation URL template from the registry is linked to the entry

  Scenario: Registry is updated from hosted JSON
    Given the current bundled registry does not contain "effect"
    When the developer runs sensei update-registry
    Then the hosted registry is fetched
    And the new "effect" entry is merged into the bundled registry
    And no existing entries are overwritten unless the remote version is newer
```

### Two-Tier Doc Retrieval

Tier 1 is a lightweight root index holding a summary per documentation page, refreshed periodically by a background daemon. Tier 2 is the on-demand page cache with semantic search enabled and a time-to-live expiry. Full crawl is opt-in per library; by default only API reference and changelog sections are targeted.

```gherkin
Feature: Two-Tier Doc Retrieval

  Scenario: Root index is populated on first doc access
    Given "svelte" is registered as a library dependency with no cached documentation yet
    When the agent calls get_lib_docs("svelte", "stores")
    Then sensei fetches the Svelte docs index page and stores a summary per discovered page
    And then fetches the "stores" page to answer the query

  Scenario: Tier 2 cache hit avoids network fetch
    Given a cached entry for svelte/stores whose TTL has not expired
    When the agent calls get_lib_docs("svelte", "stores")
    Then the cached content is returned without a network request
    And the response metadata includes when it was cached and when it expires

  Scenario: Stale cache entry is served while refreshing
    Given a cached entry for svelte/stores where the TTL has expired
    When the agent calls get_lib_docs("svelte", "stores")
    Then the stale cached content is returned immediately
    And a background refresh is triggered
    And the response metadata indicates the chunk is stale

  Scenario: Daemon refreshes root index periodically
    Given the doc index entry for "drizzle-orm" was last refreshed more than 24 hours ago
    When the sensei daemon runs its scheduled doc refresh job
    Then the Drizzle ORM docs index page is re-fetched
    And the updated summaries replace the stale ones
```

### get_lib_docs MCP Tool

`get_lib_docs` is a dedicated MCP tool for documentation retrieval, distinct from `search` which is code-first. It accepts a library name and a component or topic, consults the two-tier cache, and returns relevant doc chunks. Cache misses on small pages are resolved synchronously; larger prefetches run in the background.

```gherkin
Feature: get_lib_docs MCP Tool

  Scenario: Agent retrieves docs for a known library topic
    Given "zod" is a registered library dependency with cached documentation
    When the agent calls get_lib_docs("zod", "transform")
    Then relevant doc chunks for zod transforms are returned
    And each chunk includes: source URL, section, content, and token count

  Scenario: Cache miss on a small page is resolved synchronously
    Given no cached documentation exists for "hono/routing"
    And the hono routing page is quick to fetch
    When the agent calls get_lib_docs("hono", "routing")
    Then sensei fetches the page synchronously
    And the result is cached for future use
    And the content is returned in the same response

  Scenario: Background prefetch fires when lib imports are ranked
    Given "effect" is detected in ranked files during context_pack assembly
    And no cached documentation exists for "effect"
    When context_pack is called before any get_lib_docs call
    Then a background prefetch for the effect API reference is triggered
    And the next get_lib_docs("effect", ...) call will find a warm cache

  Scenario: get_lib_docs is distinct from search
    Given a project where "zod" is used in source files
    When the agent calls search("zod schema validation")
    Then results are code files from the project
    When the agent calls get_lib_docs("zod", "schema validation")
    Then results are zod documentation chunks, not project source files
```

### Auto-Generated Lib Skills

After indexing a custom library, a local model generates a skill markdown file summarising the library's API surface, usage patterns, and conventions. The skill is stored in `.sensei/skills/<lib-name>.md` and referenced from the Skills section of `AGENTS.md`.

```gherkin
Feature: Auto-Generated Lib Skills

  Scenario: Skill file is generated after initial custom lib index
    Given rokkit has just been indexed for the first time
    When the indexing pipeline completes
    Then .sensei/skills/rokkit.md is created
    And it contains sections: API Summary, Usage Patterns, Conventions
    And AGENTS.md Skills section references rokkit.md

  Scenario: Skill file is regenerated incrementally on source change
    Given rokkit is indexed and .sensei/skills/rokkit.md exists
    And rokkit/src/dialog.ts has been modified
    When the developer runs sensei index
    Then only the sections of rokkit.md related to the changed file are regenerated
    And unaffected sections are preserved

  Scenario: Full rescan triggers a full skill rebuild
    Given .sensei/skills/rokkit.md exists
    When the developer requests a full rescan
    Then rokkit.md is fully regenerated from all current source files
    And the previous file is replaced
```
