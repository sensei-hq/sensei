---
id: library-intelligence
type: feature
---

# Library Intelligence

> Sensei knows about the libraries you use — internal and external

Third-party libraries and internal shared packages are a major source of agent confusion: the model either hallucinates an API it doesn't know or spends tokens crawling docs it has already seen. Sensei solves this by keeping documentation indexed and readily accessible, and by generating concise skill files for custom libraries so agents work with local knowledge rather than generic training data. Every library is a first-class citizen: indexing, ranking, and context assembly treat custom libs the same as project code.

## Features

### Custom Library Indexing

Internal shared libraries are indexed from source using the same pipeline as project code. Source paths are declared in project configuration, and each library is tracked separately so ranking and slicing queries treat them consistently with project code.

```gherkin
Feature: Custom Library Indexing

  Scenario: Developer registers a custom library
    Given a custom library is configured with its source path
    When the developer runs sensei index
    Then the library's source files are scanned and parsed
    And its symbols are indexed and appear in search results alongside project code

  Scenario: Incremental re-index on file change
    Given a custom library has been indexed and one of its source files has changed
    When the developer runs sensei index
    Then only the changed file is re-parsed
    And unchanged library files are not re-processed
    And the symbol index for that file is updated

  Scenario: Force full rebuild of a custom library
    Given a custom library has been indexed
    When the developer requests a full rescan
    Then all library source files are re-parsed regardless of prior change detection
    And the complete symbol index for the library is rebuilt from scratch

  Scenario: Custom lib symbols are included in packed context
    Given a custom library is indexed and a project file imports a symbol from it
    When the agent assembles context for a task touching that import
    Then symbols from the custom library appear in the packed context
    And the response identifies them as library symbols
```

### llms.txt Priority

`llms.txt` is the highest-priority documentation source for any library. When present — either as a local file or a remote URL — sensei parses it first and supplements with source indexing only for symbols not covered by the llms.txt content.

```gherkin
Feature: llms.txt Priority

  Scenario: Remote llms.txt is fetched and parsed first
    Given a custom library configured with an llms.txt URL
    When the developer runs sensei index
    Then sensei fetches the llms.txt content before scanning source
    And documentation from llms.txt is indexed with priority
    And source indexing only runs for symbols absent from the llms.txt content

  Scenario: Bundled llms.txt file is used when a URL is not set
    Given a custom library configured with a local llms.txt path
    When the developer runs sensei index
    Then sensei reads the local llms.txt file
    And its content is parsed and indexed before source files are scanned

  Scenario: llms.txt chunks take precedence in search results
    Given a library indexed with both llms.txt content and source symbols
    When the agent retrieves documentation for that library
    Then chunks sourced from llms.txt are ranked above source-derived chunks
    And the response indicates each chunk's origin
```

### External Doc Registry

The bundled library registry maps known libraries to their documentation URL templates and section structure. The registry is kept up to date and accepts community contributions. Per-repo library usage is tracked and doc content is cached for reuse.

```gherkin
Feature: External Doc Registry

  Scenario: Registry entry drives doc URL resolution
    Given the library registry contains an entry for "zod" with a documentation URL template
    And the project lists "zod" as a dependency
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

Documentation retrieval uses a layered approach: a fast index of page summaries for quick lookup, and a deeper on-demand cache for full content retrieval with semantic search. Full crawl is opt-in per library; by default only the most relevant sections are targeted.

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

A dedicated documentation retrieval tool is distinct from code search. It accepts a library name and a component or topic and returns relevant doc chunks. Requests are resolved as quickly as possible, with background prefetching used when content is not yet available.

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
    Given "effect" is detected in ranked files during context assembly
    And no cached documentation exists for "effect"
    When context is assembled before any get_lib_docs call
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

After indexing a custom library, sensei generates a skill file summarising the library's API surface, usage patterns, and conventions. The skill is referenced from the project's shared agent context so all configured agents benefit from it.

```gherkin
Feature: Auto-Generated Lib Skills

  Scenario: Skill file is generated after initial custom lib index
    Given a custom library has just been indexed for the first time
    When the indexing pipeline completes
    Then a skill file is created for that library
    And it contains sections: API Summary, Usage Patterns, Conventions
    And the AGENTS.md Skills section references the new skill file

  Scenario: Skill file is regenerated incrementally on source change
    Given a custom library is indexed and its skill file exists
    And one of its source files has been modified
    When the developer runs sensei index
    Then only the sections of the skill file related to the changed source are regenerated
    And unaffected sections are preserved

  Scenario: Full rescan triggers a full skill rebuild
    Given a custom library's skill file exists
    When the developer requests a full rescan
    Then the skill file is fully regenerated from all current source files
    And the previous file is replaced
```
