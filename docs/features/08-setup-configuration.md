---
id: setup-configuration
type: feature
---

# Setup and Configuration

> Sensei is one command to get started and adapts to your team

Getting a new tool adopted depends almost entirely on how hard the first five minutes are. Sensei is designed so that `sensei init` handles detection, registration, indexing, and agent wiring in a single command, leaving the developer with a working setup before they have time to lose interest. Beyond the initial setup, every significant decision — which model to use for embedding, how to rank context, which agents are active — is configurable at the global, project, and per-agent level without forcing a rewrite of anything already working.

## Features

### sensei init

A single command that detects the project stack, registers the repo in Supabase, runs the initial indexing pipeline, generates `AGENTS.md`, installs agent adapters for all detected agents, and writes `.sensei/config.yaml`. There is no migration path — the project is pre-release and always starts fresh.

```gherkin
Feature: sensei init

  Scenario: Developer initialises a new repo with local Supabase
    Given a code repository with package.json and no .sensei/ directory
    And supabase start has been run and local Supabase is available
    When the developer runs sensei init
    Then the repo is registered in the local Supabase instance
    And the initial indexing pipeline runs and indexes all source files
    And AGENTS.md is created at the repo root
    And .sensei/config.yaml is written with mode: local

  Scenario: Developer initialises with cloud Supabase
    Given the developer has set SUPABASE_URL and SUPABASE_ANON_KEY environment variables
    When the developer runs sensei init
    Then the repo is registered in the cloud Supabase project
    And .sensei/config.yaml is written with mode: cloud

  Scenario: Agent adapters are installed for detected agents
    Given a repo with .claude/ and .cursor/ directories already present
    When the developer runs sensei init
    Then sensei installs skills to .claude/skills/ and .cursor/skills/
    And PreToolUse, PostToolUse, and UserPromptSubmit hooks are registered for each agent
    And AGENTS.md is updated to reference the installed skills

  Scenario: Init is idempotent when run a second time
    Given a repo where sensei init has already been run
    When the developer runs sensei init again
    Then no duplicate Supabase registrations are created
    And existing .sensei/config.yaml is preserved unless --force is passed
    And a summary of what was updated vs skipped is printed
```

### Deployment Modes

Sensei supports local deployment via `supabase start` (Docker) and cloud deployment via supabase.com. Both modes use the same Supabase client. The mode is set in `.sensei/config.yaml`. pgvector ships by default with `supabase start`. Supabase is always required — there is no fallback to `.sensei/` JSON files.

```gherkin
Feature: Deployment Modes

  Scenario: Local mode connects to supabase start
    Given .sensei/config.yaml has mode: local
    When any sensei command that writes to the database runs
    Then the Supabase client connects to http://localhost:54321
    And pgvector queries execute successfully against the local instance

  Scenario: Cloud mode connects to supabase.com
    Given .sensei/config.yaml has mode: cloud and SUPABASE_URL is set to a cloud project URL
    When any sensei command that writes to the database runs
    Then the Supabase client connects to the cloud project URL
    And the same schema and queries work without modification

  Scenario: Missing Supabase connection fails with a clear error
    Given .sensei/config.yaml has mode: local but supabase start is not running
    When the developer runs sensei index
    Then the command exits with an error: "Cannot connect to local Supabase. Run: supabase start"
    And no fallback to JSON files is attempted
```

### Model Configuration Hierarchy

Global model configuration lives in `~/.config/sensei/config.yaml` and covers inference provider, base URL, and model assignments for embedding, indexing, extraction, classification, and default tasks. Project overrides in `.sensei/config.yaml` can replace individual model assignments without restating the full config. The `ModelBackend` interface is provider-agnostic, and `TokenCounter` selects the appropriate counting strategy per provider.

```gherkin
Feature: Model Configuration Hierarchy

  Scenario: Project config overrides global embedding model
    Given ~/.config/sensei/config.yaml sets embedding model to "nomic-embed-text"
    And .sensei/config.yaml overrides embedding_model to "mxbai-embed-large"
    When sensei runs an embedding operation
    Then "mxbai-embed-large" is used for this project
    And "nomic-embed-text" remains the default for other projects

  Scenario: Global config is used when no project override exists
    Given ~/.config/sensei/config.yaml sets default_model to "llama3.2"
    And .sensei/config.yaml does not specify a default_model
    When sensei runs a classification task for this project
    Then "llama3.2" is used as the model

  Scenario: TokenCounter selects strategy based on provider
    Given the configured model is claude-3-5-sonnet via the Anthropic provider
    When sensei counts tokens for a context pack
    Then the Anthropic API tokenizer is used
    And the count is accurate to within 1% of the actual API token count

  Scenario: OpenAI-compatible provider is used via ModelBackend interface
    Given .sensei/config.yaml sets provider to "lm-studio" with base_url "http://localhost:1234/v1"
    When sensei calls the indexing model
    Then the request is sent to http://localhost:1234/v1 using the OpenAI-compatible client
    And the response is handled identically to any other ModelBackend provider
```

### Agent Adapter Setup

`sensei setup` detects which agents are configured for the repo by checking for agent-specific directories (`.claude/`, `.cursor/`, `.opencode/`, etc.), installs skills to the appropriate locations, writes agent-specific configuration files, and registers all required hooks. A specific agent can be added with `sensei setup --agent <name>`.

```gherkin
Feature: Agent Adapter Setup

  Scenario: Setup detects and configures all present agents
    Given a repo with .claude/ and .opencode/ directories
    When the developer runs sensei setup
    Then skills are installed to .claude/skills/ and .opencode/skills/
    And hooks are registered for both Claude and OpenCode
    And a confirmation lists each agent configured and the files written

  Scenario: Single agent is added explicitly
    Given a repo where only Claude is configured
    When the developer runs sensei setup --agent cursor
    Then .cursor/ is created if absent
    And skills are installed to .cursor/skills/
    And Cursor-specific hook configuration is written

  Scenario: Re-running setup updates hooks without duplicating them
    Given sensei setup has already been run and hooks are installed
    When the developer runs sensei setup again
    Then no duplicate hook entries are added to agent config files
    And any updated skill files are written with their new content
```

### Ranking Strategy Configuration

Each repo can define its own ranking chain in `.sensei/config.yaml` as an ordered list of strategies. Sensible defaults apply when no chain is specified. Strategies can be disabled for latency-sensitive setups.

```gherkin
Feature: Ranking Strategy Configuration

  Scenario: Default ranking chain is used when none is configured
    Given .sensei/config.yaml has no ranking_chain entry
    When the agent calls context_pack
    Then the default chain runs: diff_first_bfs → traceability_boost → external_docs → semantic → bm25
    And results are returned within the configured token budget

  Scenario: Project disables semantic ranking for latency
    Given .sensei/config.yaml sets ranking_chain to [diff_first_bfs, bm25]
    When the agent calls context_pack
    Then no semantic embedding query is made
    And the response latency is lower than with the default chain
    And results are still ranked by diff relevance and BM25 score

  Scenario: Traceability boost elevates linked files
    Given the ranking_chain includes traceability_boost
    And the current task touches a file linked to a feature doc in sensei.traceability
    When the agent calls context_pack
    Then the linked feature doc appears in the top 3 results
    And its rank position is higher than its BM25 or semantic score alone would produce
```

### Library Registry Updates

`sensei update-registry` fetches the latest `lib_registry` from a hosted JSON endpoint and merges new library entries into the bundled list. Custom libraries are registered in `.sensei/config.yaml` with a source path or `llms_txt_url` and an optional skill generation flag.

```gherkin
Feature: Library Registry Updates

  Scenario: Registry is updated from the hosted JSON
    Given the current bundled registry does not include "vinxi"
    When the developer runs sensei update-registry
    Then the hosted registry JSON is fetched
    And the "vinxi" entry is added to the local registry
    And existing entries are preserved unless the remote version field is newer

  Scenario: Custom library is registered in project config
    Given .sensei/config.yaml has a custom_libs entry for "dbd" with source_path "../dbd/src" and generate_skill: true
    When the developer runs sensei index
    Then "dbd" source files are scanned and indexed
    And .sensei/skills/dbd.md is generated after indexing completes

  Scenario: Custom library with llms_txt_url skips source scan for covered symbols
    Given .sensei/config.yaml registers "kavach" with llms_txt_url "https://kavach.dev/llms.txt"
    When the developer runs sensei index
    Then kavach's llms.txt is fetched and parsed first
    And source indexing runs only for kavach symbols absent from the llms.txt content
```

### Project Config Editing

Layer 2 metadata (`project_config`) is editable via the dashboard or CLI. Fields include project description, team guidelines, custom context, and agent preferences. Editing any field triggers skill regeneration for affected agents.

```gherkin
Feature: Project Config Editing

  Scenario: Developer updates team guidelines via CLI
    Given a project registered in Supabase
    When the developer runs sensei config set guidelines "Always write tests before implementation"
    Then the project_config record in Supabase is updated with the new guideline
    And affected skill files are queued for regeneration

  Scenario: Skill files are regenerated after config change
    Given a project_config change updates the "agent_preferences" field
    When the regeneration job runs
    Then AGENTS.md is rewritten to reflect the updated preferences
    And .claude/skills/project.md is updated with the new content
    And unchanged skill files are not rewritten

  Scenario: Dashboard edits persist to Supabase
    Given the developer opens the project settings view in the dashboard
    When they update the project description and click Save
    Then the project_config row in Supabase is updated immediately
    And a toast notification confirms the save and notes that skill regeneration has been queued
```
