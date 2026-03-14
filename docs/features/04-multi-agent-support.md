---
id: multi-agent-support
type: feature
---

# Multi-Agent Support

> Sensei works with Claude, Cursor, opencode, and any agent you use

Each AI coding agent has its own conventions for loading project context and skills. Without a shared layer, you'd have to maintain duplicate context files for every agent you use. Sensei provides a single source of truth — the three-layer metadata model — and installs the right files in the right places for each agent automatically.

## Features

### Agent Adapter Pattern

Sensei integrates with each agent through a dedicated adapter. A generic fallback handles agents with no structured skills concept. Each adapter is responsible for installing skills to the agent's expected location and format.

```gherkin
Feature: Agent Adapter Pattern

  Scenario: Skills are installed to the Claude Code skills directory
    Given a repo with generated skills and Claude Code configured
    When the Claude adapter runs
    Then skill files are written to the location expected by Claude Code
    And each file uses the format expected by Claude Code

  Scenario: Skills are installed as Cursor rules
    Given a repo with generated skills and Cursor configured
    When the Cursor adapter runs
    Then skill files are written to the location expected by Cursor
    And each file follows the format Cursor expects for rules

  Scenario: Generic fallback writes skill references into AGENTS.md
    Given a repo configured for an agent with no native skills concept
    When the generic adapter runs
    Then skill summaries are appended to the Skills section of AGENTS.md
    And no agent-specific directory is created

  Scenario: Adapter installation is idempotent
    Given an adapter that has already run and installed skills
    When the adapter runs again with no changes to the source skills
    Then no files are written or modified
    And the existing skill files are preserved unchanged
```

### AGENTS.md

`AGENTS.md` is the universal project context file. It uses structured H2 sections — Goals, Stack, Guidelines, Patterns, Skills — that all agents can read as project context regardless of their native format. Agents that support a primary context file can reference `AGENTS.md` from it for the full project context. Sensei writes `AGENTS.md` on first setup and keeps it updated whenever project configuration changes.

```gherkin
Feature: AGENTS.md

  Scenario: sensei init writes AGENTS.md with all standard sections
    Given a freshly indexed repo with a populated project profile and user configuration
    When the developer runs sensei init
    Then AGENTS.md is created at the repo root
    And it contains H2 sections for Goals, Stack, Guidelines, Patterns, and Skills
    And each section is populated from the corresponding layer of the metadata model

  Scenario: CLAUDE.md references AGENTS.md for dual-file agents
    Given a repo where both CLAUDE.md and AGENTS.md are present
    When an agent that supports CLAUDE.md reads the project context
    Then CLAUDE.md contains a reference to AGENTS.md
    And the agent can follow the reference to read the full project context

  Scenario: AGENTS.md is updated when user configuration changes
    Given an AGENTS.md with a Guidelines section populated from user configuration
    And the user has updated their guidelines in the dashboard
    When the indexer detects the configuration change
    Then AGENTS.md is rewritten with the updated Guidelines section
    And all other sections remain unchanged

  Scenario: Any agent can consume AGENTS.md as plain context
    Given an agent with no native skills or rules concept
    When the agent reads AGENTS.md at the start of a session
    Then it receives the project goals, stack, guidelines, patterns, and skill summaries
    And no agent-specific installation step is required
```

### Skills Installation

Skills are generated from the combined auto-extracted project profile and user-authored project configuration. They are not templated — each skill is authored from the actual project context. Generated skills are installed by the appropriate adapter to each agent's expected location. Skills are regenerated whenever the profile or config changes, using the same trigger as incremental indexing.

```gherkin
Feature: Skills Installation

  Scenario: Skills are generated from project profile and config
    Given a repo with a populated project profile and user configuration
    When skill generation runs
    Then skill files are generated and stored
    And each skill's content reflects the actual project stack, patterns, and guidelines
    And no generic template text appears in the output

  Scenario: Skills are installed to the correct agent-specific location
    Given generated skills and Claude Code configured for the repo
    When the Claude adapter installs skills
    Then skills are written to the location expected by Claude Code
    Given opencode is also configured for the repo
    When the opencode adapter installs skills
    Then skills are written to the location expected by opencode

  Scenario: Skills are regenerated when profile or config changes
    Given skills generated from the previous configuration
    And the user has added two new guidelines
    When the indexer detects the config change
    Then skill generation re-runs with the updated data
    And the new skills replace the previous ones
    And each configured adapter re-installs the updated skills to its target directory

  Scenario: Skill generation does not run when nothing has changed
    Given current skills with no profile or config changes since last generation
    When the indexer runs
    Then skill generation is skipped
    And the existing skill files are left untouched
```

### Agent Config and Capability Detection

Per-repo agent preferences are stored persistently. When installing context files, Sensei checks which agents are configured for the repo and dispatches to the appropriate adapters. Agents that have no native skills or rules concept are handled by a generic fallback, which surfaces skill references through `AGENTS.md` instead.

```gherkin
Feature: Agent Config and Capability Detection

  Scenario: Configured agents are detected and dispatched
    Given a repo where Claude Code and Cursor are configured as agents
    When Sensei runs the skills installation step
    Then skills are installed to the Claude Code skills directory
    And skills are installed to the Cursor rules directory
    And no other adapters are invoked

  Scenario: Unconfigured agents are not installed
    Given a repo where only Claude Code is configured
    When Sensei runs the skills installation step
    Then the Cursor adapter does not run
    And no Cursor-specific directories are created

  Scenario: Agent without skills concept falls back to generic adapter
    Given a repo configured for an agent that has no native skills or rules directory
    When Sensei detects this agent during capability detection
    Then the generic adapter is selected for that agent
    And skill summaries are written into the Skills section of AGENTS.md

  Scenario: Developer adds a new agent to the configuration
    Given a repo previously configured for Claude Code only
    When the developer adds opencode to the agent configuration
    And runs sensei install-skills
    Then the opencode adapter runs and installs skills to the opencode skills directory
    And the Claude Code adapter re-runs to ensure its skills are current
```

### Token Usage Tracking Per Agent

Every task session records the agent name and version. Every turn records the model and provider, handling mid-session model switches. This data enables cross-agent quality comparisons and per-model cost analytics in the dashboard.

```gherkin
Feature: Token Usage Tracking Per Agent

  Scenario: Session records agent name and version
    Given an agent starting a new task session
    When the session begins
    Then agent name and version are recorded
    And these fields are set from the agent's self-reported identity or config

  Scenario: Each turn records model and provider
    Given an active task session
    When the agent makes a turn using a specific model from a provider
    Then the model and provider are recorded for that turn
    And the token counts for that turn are recorded against that model

  Scenario: Mid-session model switch is captured per turn
    Given a session that started with one model and switched to another on turn 5
    When token usage is aggregated for the session
    Then turns 1-4 are attributed to the first model
    And turns 5+ are attributed to the second model
    And the session total correctly sums tokens across both models

  Scenario: Cross-agent quality comparison is available in analytics
    Given 30 sessions across Claude Code and Cursor working on the same repo
    When the developer views analytics in the dashboard
    Then token usage per task type is broken down by agent and model
    And context hit rates are shown per agent
    And the developer can compare which agent is most token-efficient for each task type
```
