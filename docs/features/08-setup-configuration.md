---
id: setup-configuration
type: feature
---

# Setup and Configuration

> Sensei is one command to get started and adapts to your team

Getting a new tool adopted depends almost entirely on how hard the first five minutes are. Sensei is designed so that `sensei init` handles detection, registration, indexing, and agent wiring in a single command, leaving the developer with a working setup before they have time to lose interest. Beyond the initial setup, every significant decision — which model to use for embedding, how to rank context, which agents are active — is configurable at the global, project, and per-agent level without forcing a rewrite of anything already working.

## Features

### sensei init

A single command that detects the project stack, registers the repo, runs the initial indexing pipeline, generates `AGENTS.md`, installs agent adapters for all detected agents, and writes `.sensei/config.yaml`. There is no migration path — the project is pre-release and always starts fresh.

```gherkin
Feature: sensei init

  Scenario: Developer initialises a new repo in local mode
    Given a code repository with package.json and no .sensei/ directory
    And a local backend instance is running
    When the developer runs sensei init
    Then the repo is registered with the local backend
    And the initial indexing pipeline runs and indexes all source files
    And AGENTS.md is created at the repo root
    And .sensei/config.yaml is written with mode: local

  Scenario: Developer initialises in cloud mode
    Given the developer has configured cloud backend credentials in their environment
    When the developer runs sensei init
    Then the repo is registered with the cloud backend
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
    Then no duplicate registrations are created
    And existing .sensei/config.yaml is preserved unless a force option is passed
    And a summary of what was updated vs skipped is printed
```

### Deployment Modes

Sensei supports local deployment (running entirely on your machine) and cloud deployment. Both modes use the same configuration format and feature set. The mode is set in `.sensei/config.yaml`. A backend is always required — there is no fallback to local files only.

```gherkin
Feature: Deployment Modes

  Scenario: Local mode connects to the local backend
    Given .sensei/config.yaml has mode: local
    When any sensei command that writes data runs
    Then the client connects to the local backend instance
    And all queries execute successfully against the local instance

  Scenario: Cloud mode connects to the cloud backend
    Given .sensei/config.yaml has mode: cloud and cloud credentials are configured
    When any sensei command that writes data runs
    Then the client connects to the cloud backend
    And the same features and queries work without modification

  Scenario: Missing backend connection fails with a clear error
    Given .sensei/config.yaml has mode: local but the local backend is not running
    When the developer runs sensei index
    Then the command exits with a clear error explaining how to start the local backend
    And no fallback to local files only is attempted
```

### Model Configuration Hierarchy

Global model configuration lives in `~/.config/sensei/config.yaml` and covers inference provider, base URL, and model assignments for embedding, indexing, extraction, classification, and default tasks. Project overrides in `.sensei/config.yaml` can replace individual model assignments without restating the full config. The model backend is provider-agnostic, and token counting selects the appropriate strategy per provider.

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

  Scenario: Token counting selects strategy based on provider
    Given the configured model is from the Anthropic provider
    When sensei counts tokens for a context pack
    Then the provider-appropriate tokenization strategy is used
    And the count is accurate to within 1% of the actual API token count

  Scenario: OpenAI-compatible provider is used transparently
    Given .sensei/config.yaml sets an OpenAI-compatible provider with its base URL
    When sensei calls the indexing model
    Then the request is sent to the configured endpoint using the compatible client
    And the response is handled identically to any other provider
```

### Agent Adapter Setup

`sensei setup` detects which agents are configured for the repo by checking for agent-specific directories, installs skills to the appropriate locations, writes agent-specific configuration files, and registers all required hooks. A specific agent can be added with `sensei setup --agent <name>`.

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
    Then the default chain runs: diff-first traversal, traceability boost, external docs, semantic search, and keyword ranking
    And results are returned within the configured token budget

  Scenario: Project disables semantic ranking for latency
    Given .sensei/config.yaml sets the ranking chain to diff-first traversal and keyword ranking only
    When the agent calls context_pack
    Then no semantic search is performed
    And the response latency is lower than with the default chain
    And results are still ranked by diff relevance and keyword score

  Scenario: Traceability boost elevates linked files
    Given the ranking chain includes traceability boost
    And the current task touches a file linked to a feature doc
    When the agent calls context_pack
    Then the linked feature doc appears in the top 3 results
    And its rank position is higher than its traversal or semantic score alone would produce
```

### Library Registry Updates

`sensei update-registry` fetches the latest library registry from a hosted source and merges new library entries into the bundled list. Custom libraries are registered in `.sensei/config.yaml` with a source path or llms.txt URL and an optional skill generation flag.

```gherkin
Feature: Library Registry Updates

  Scenario: Registry is updated from the hosted JSON
    Given the current bundled registry does not include "vinxi"
    When the developer runs sensei update-registry
    Then the hosted registry JSON is fetched
    And the "vinxi" entry is added to the local registry
    And existing entries are preserved unless the remote version field is newer

  Scenario: Custom library is registered in project config
    Given .sensei/config.yaml has a custom_libs entry for "dbd" with a source path and skill generation enabled
    When the developer runs sensei index
    Then "dbd" source files are scanned and indexed
    And .sensei/skills/dbd.md is generated after indexing completes

  Scenario: Custom library with llms.txt URL skips source scan for covered symbols
    Given .sensei/config.yaml registers "kavach" with a configured llms.txt URL
    When the developer runs sensei index
    Then kavach's llms.txt is fetched and parsed first
    And source indexing runs only for kavach symbols absent from the llms.txt content
```

### Project Config Editing

User-authored project configuration is editable via the dashboard or CLI. Fields include project description, team guidelines, custom context, and agent preferences. Editing any field triggers skill regeneration for affected agents.

```gherkin
Feature: Project Config Editing

  Scenario: Developer updates team guidelines via CLI
    Given a registered project
    When the developer runs sensei config set guidelines "Always write tests before implementation"
    Then the project configuration is updated with the new guideline
    And affected skill files are queued for regeneration

  Scenario: Skill files are regenerated after config change
    Given a project configuration change updating the agent preferences
    When the regeneration job runs
    Then AGENTS.md is rewritten to reflect the updated preferences
    And installed skill files are updated with the new content
    And unchanged skill files are not rewritten

  Scenario: Dashboard edits persist immediately
    Given the developer opens the project settings view in the dashboard
    When they update the project description and click Save
    Then the project configuration is updated immediately
    And a toast notification confirms the save and notes that skill regeneration has been queued
```
