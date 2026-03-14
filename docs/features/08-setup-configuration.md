---
id: setup-configuration
type: feature
---

# Setup and Configuration

> Sensei is one command to get started and adapts to your team

Getting a new tool adopted depends almost entirely on how hard the first five minutes are. Sensei is designed so that a single init command handles detection, registration, indexing, and agent wiring, leaving the developer with a working setup before they have time to lose interest. Beyond the initial setup, every significant decision — which model to use for embedding, how to rank context, which agents are active — is configurable at the global, project, and per-agent level without forcing a rewrite of anything already working.

## Features

### sensei init

A single command that detects the project stack, registers the repo, runs the initial indexing pipeline, generates agent guidance files, installs agent adapters for all detected agents, and writes the project configuration.

```gherkin
Feature: sensei init

  Scenario: Developer initialises a new repo in local mode
    Given a code repository with no existing sensei configuration
    And a local backend instance is running
    When the developer runs the init command
    Then the repo is registered with the local backend
    And the initial indexing pipeline runs and indexes all source files
    And agent guidance files are created at the repo root
    And the project configuration is written for local mode

  Scenario: Developer initialises in cloud mode
    Given the developer has configured cloud backend credentials in their environment
    When the developer runs the init command
    Then the repo is registered with the cloud backend
    And the project configuration is written for cloud mode

  Scenario: Agent adapters are installed for detected agents
    Given a repo with directories for multiple agents already present
    When the developer runs the init command
    Then sensei installs skills to the appropriate location for each detected agent
    And hooks are registered for each agent
    And agent guidance files are updated to reference the installed skills

  Scenario: Init is idempotent when run a second time
    Given a repo where init has already been run
    When the developer runs the init command again
    Then no duplicate registrations are created
    And existing project configuration is preserved unless a force option is passed
    And a summary of what was updated vs skipped is printed
```

### Deployment Modes

Sensei supports local deployment (running entirely on your machine) and cloud deployment. Both modes use the same configuration format and feature set. The mode is set in the project configuration file.

```gherkin
Feature: Deployment Modes

  Scenario: Local mode connects to the local backend
    Given the project is configured for local mode
    When any sensei command that writes data runs
    Then the client connects to the local backend instance
    And all queries execute successfully against the local instance

  Scenario: Cloud mode connects to the cloud backend
    Given the project is configured for cloud mode and cloud credentials are configured
    When any sensei command that writes data runs
    Then the client connects to the cloud backend
    And the same features and queries work without modification

  Scenario: Missing backend connection fails with a clear error
    Given the project is configured for local mode but the local backend is not running
    When the developer runs an indexing command
    Then the command exits with a clear error explaining how to start the local backend
    And no fallback to local files only is attempted
```

### Model Configuration Hierarchy

Global model configuration covers inference provider, base URL, and model assignments for embedding, indexing, extraction, classification, and default tasks. Project-level overrides can replace individual model assignments without restating the full config. Sensei is provider-agnostic and works with any compatible inference provider.

```gherkin
Feature: Model Configuration Hierarchy

  Scenario: Project config overrides global embedding model
    Given the global config sets a default embedding model
    And the project config overrides it with a different model
    When sensei runs an embedding operation
    Then the project-level model is used for this project
    And the global default remains in effect for other projects

  Scenario: Global config is used when no project override exists
    Given the global config sets a default model
    And the project config does not specify a model for a given task
    When sensei runs a classification task for this project
    Then the global default model is used

  Scenario: Token counting selects strategy based on provider
    Given a provider is configured
    When sensei counts tokens for a context pack
    Then the provider-appropriate tokenization strategy is used
    And the count is accurate to within 1% of the actual API token count

  Scenario: OpenAI-compatible provider is used transparently
    Given an OpenAI-compatible provider is configured with its base URL
    When sensei calls the indexing model
    Then the request is sent to the configured endpoint using the compatible client
    And the response is handled identically to any other provider
```

### Agent Adapter Setup

The setup command detects which agents are configured for the repo, installs skills to the appropriate locations, writes agent-specific configuration files, and registers all required hooks. A specific agent can also be added explicitly by name.

```gherkin
Feature: Agent Adapter Setup

  Scenario: Setup detects and configures all present agents
    Given a repo with directories for multiple agents
    When the developer runs the setup command
    Then skills are installed to the appropriate location for each detected agent
    And hooks are registered for each agent
    And a confirmation lists each agent configured and the files written

  Scenario: Single agent is added explicitly
    Given a repo where only one agent is configured
    When the developer runs setup for a specific additional agent
    Then the agent's directory is created if absent
    And skills are installed to the correct location for that agent
    And agent-specific hook configuration is written

  Scenario: Re-running setup updates hooks without duplicating them
    Given setup has already been run and hooks are installed
    When the developer runs the setup command again
    Then no duplicate hook entries are added to agent config files
    And any updated skill files are written with their new content
```

### Ranking Strategy Configuration

Each repo can define its own ranking chain in the project configuration as an ordered list of strategies. Sensible defaults apply when no chain is specified. Strategies can be disabled for latency-sensitive setups.

```gherkin
Feature: Ranking Strategy Configuration

  Scenario: Default ranking chain is used when none is configured
    Given no ranking chain is specified in the project configuration
    When the agent requests a context pack
    Then the default chain runs: diff-first traversal, traceability boost, external docs, semantic search, and keyword ranking
    And results are returned within the configured token budget

  Scenario: Project disables semantic ranking for latency
    Given the project ranking chain is configured to use only diff-first traversal and keyword ranking
    When the agent requests a context pack
    Then no semantic search is performed
    And the response latency is lower than with the default chain
    And results are still ranked by diff relevance and keyword score

  Scenario: Traceability boost elevates linked files
    Given the ranking chain includes traceability boost
    And the current task touches a file linked to a feature doc
    When the agent requests a context pack
    Then the linked feature doc appears in the top results
    And its rank position is higher than its traversal or semantic score alone would produce
```

### Library Registry Updates

A registry update command fetches the latest library registry and merges new library entries into the bundled list. Custom libraries are registered in the project configuration with a source path or documentation URL and an optional skill generation flag.

```gherkin
Feature: Library Registry Updates

  Scenario: Registry is updated from the hosted source
    Given the current bundled registry does not include a recently released library
    When the developer runs the registry update command
    Then the hosted registry is fetched
    And the new library entry is added to the local registry
    And existing entries are preserved unless the remote version is newer

  Scenario: Custom library is registered in project config
    Given a custom library is registered in project config with a source path and skill generation enabled
    When the developer runs the indexing command
    Then the library's source files are scanned and indexed
    And a skill file is generated for that library after indexing completes

  Scenario: Custom library with documentation URL skips source scan for covered symbols
    Given a custom library is registered with a documentation URL
    When the developer runs the indexing command
    Then the documentation URL is fetched and parsed first
    And source indexing runs only for symbols absent from the documentation content
```

### Project Config Editing

User-authored project configuration is editable via the dashboard or CLI. Fields include project description, team guidelines, custom context, and agent preferences. Editing any field triggers skill regeneration for affected agents.

```gherkin
Feature: Project Config Editing

  Scenario: Developer updates team guidelines via CLI
    Given a registered project
    When the developer sets a new team guideline via the config command
    Then the project configuration is updated with the new guideline
    And affected skill files are queued for regeneration

  Scenario: Skill files are regenerated after config change
    Given a project configuration change updating the agent preferences
    When the regeneration job runs
    Then agent guidance files are rewritten to reflect the updated preferences
    And installed skill files are updated with the new content
    And unchanged skill files are not rewritten

  Scenario: Dashboard edits persist immediately
    Given the developer opens the project settings view in the dashboard
    When they update the project description and click Save
    Then the project configuration is updated immediately
    And a toast notification confirms the save and notes that skill regeneration has been queued
```
