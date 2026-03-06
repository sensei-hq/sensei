# Agentic Dev Workflow

An AI developer agent without a workflow protocol accumulates context, repeats work, and makes broad searches where targeted ones would suffice. The agentic dev workflow defines how agents should start sessions, load context, work on tasks, and transition between them — maximising usefulness per token.

## Features

### Session Orientation Protocol

Every session starts the same way: orientation via the LLMSpec, not via file reads.

```gherkin
Feature: Session Orientation Protocol

  Scenario: Agent orients at session start
    Given an indexed repo
    When an agent begins a new session
    Then the agent calls get_llmspec() before reading any files
    And receives project name, description, stack, entry points, and shortcuts
    And spends under 600 tokens on orientation

  Scenario: Agent does not read files to orient
    Given an indexed repo
    When an agent is asked to start working
    Then the agent does not call Read or Glob before loading the LLMSpec
    And uses get_llmspec() as the first action

  Scenario: Agent uses recommend_next before loading context
    Given an agent that has oriented via LLMSpec
    And has received a task
    When the agent prepares to work
    Then the agent calls recommend_next(task) before loading any files
    And loads only the prescribed context slice
```

### Targeted Context Loading

Agents load exactly the context slice prescribed for their task — no more.

```gherkin
Feature: Targeted Context Loading

  Scenario: Agent loads orientation slice for a new task
    Given an agent starting a new task on an indexed repo
    When the agent calls load_context("orientation")
    Then a compact project summary is returned
    And total tokens are under 250

  Scenario: Agent loads module slice for a focused task
    Given an agent working on the auth module
    When the agent calls load_context("src/auth")
    Then only exports from the auth module are returned at L0
    And exports from other modules are not included

  Scenario: Agent loads patterns before editing
    Given an agent about to add a new function
    When the agent calls load_context("patterns")
    Then the repo's coding conventions are returned
    And the agent can follow existing patterns without guessing

  Scenario: Agent does not load full file trees
    Given an indexed repo
    When an agent needs to understand module structure
    Then the agent calls list_exports(module) rather than reading directories
    And total tokens for the operation are under 150
```

### MCP Offload Protocol

Deterministic, repeatable tasks go to MCP tools — not the LLM's reasoning loop.

```gherkin
Feature: MCP Offload Protocol

  Scenario: Agent generates llms.txt via MCP
    Given an indexed repo
    When an agent needs to produce llms.txt
    Then the agent calls generate_llms_txt() rather than writing it from scratch
    And the output is consistent with the current index
    And no tokens are spent generating the content in-context

  Scenario: Agent checks drift via MCP
    Given an indexed repo where docs may have changed
    When an agent needs to know if docs are in sync
    Then the agent calls check_drift() rather than comparing files manually
    And receives a structured drift report in one tool call

  Scenario: Agent lists exports via MCP
    Given an indexed repo
    When an agent needs to know what functions a module exports
    Then the agent calls list_exports(module) rather than reading the source file
    And receives L0 signatures for all exports

  Scenario: Agent finds patterns via MCP
    Given an indexed repo with detected patterns
    When an agent needs to know the DB access pattern
    Then the agent calls find_pattern("repository") rather than searching source files
    And receives the pattern description in one tool call
```

### Task Transition Protocol

Agents checkpoint before switching tasks to avoid carrying stale context.

```gherkin
Feature: Task Transition Protocol

  Scenario: Agent checkpoints before switching tasks
    Given an agent that has completed a task
    And is about to start a different task
    When the agent transitions
    Then the agent calls checkpoint() before starting the new task
    And then calls recommend_next(new_task) for fresh context prescription

  Scenario: Checkpoint saves context state
    Given an agent that has loaded context for task A
    When the agent calls checkpoint("task-a-complete")
    Then the current context state is saved
    And can be referenced later if needed

  Scenario: Agent does not carry previous context into new task
    Given an agent that has finished working on the auth module
    And is now asked to work on the billing module
    When the agent starts the billing task
    Then the agent calls checkpoint() to mark the transition
    And calls load_context("src/billing") for the new scope
    And does not keep auth module content in context
```

## Status

| Feature | Status |
|---------|--------|
| Session orientation protocol (LLMSpec first) | 🔲 Planned |
| Targeted context loading | 🔲 Planned |
| MCP offload protocol | 🔲 Planned |
| Task transition with checkpoint | 🔲 Planned |
| recommend_next task-to-context prescription | 🔲 Planned |
| Anti-pattern guidance (what NOT to do) | 🔲 Planned |
