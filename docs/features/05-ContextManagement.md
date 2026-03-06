# Context Management

The context window is working memory. When it fills with stale content from previous tasks, agents make mistakes — referencing things that are no longer in scope, missing relevant current state, or spending tokens on content they won't use. Context management defines how agents load, maintain, and release context across a session.

## Features

### Targeted Slice Loading

Agents load a named slice — orientation, patterns, or a module path — and receive only what belongs in that scope.

```gherkin
Feature: Targeted Slice Loading

  Scenario: Agent loads orientation slice
    Given an indexed repo
    When the agent calls load_context("orientation")
    Then the response contains project name, description, stack, and entry points
    And the total tokens are under 250
    And no module exports are included

  Scenario: Agent loads a module slice
    Given an indexed repo with a src/payments module
    When the agent calls load_context("src/payments")
    Then the response contains L0 signatures for all exports in src/payments/
    And exports from other modules are not included
    And total tokens are under 150 for a typical module

  Scenario: Agent loads patterns slice
    Given an indexed repo with detected code patterns
    When the agent calls load_context("patterns")
    Then the response contains the patterns document
    And does not include code or documentation content

  Scenario: Unknown scope is handled gracefully
    Given an indexed repo
    When the agent calls load_context("src/nonexistent")
    Then the response reports no exports found for that scope
    And does not error
```

### Token Budget Reporting

Each context slice includes a token estimate so agents can reason about their budget.

```gherkin
Feature: Token Budget Reporting

  Scenario: Slice includes token estimate
    Given an indexed repo
    When the agent calls load_context("src/auth")
    Then the response includes a token estimate
    And the estimate reflects the size of the returned content

  Scenario: Agent can compare slice costs before loading
    Given an indexed repo
    When the agent calls get_context_summary()
    Then the response lists available scopes with estimated token costs
    And the agent can choose the most targeted scope for its task
```

### Checkpoint and Restore

Agents save context state before switching tasks and can reference checkpoints later.

```gherkin
Feature: Checkpoint and Restore

  Scenario: Agent creates a named checkpoint
    Given an agent that has completed work on the auth module
    When the agent calls checkpoint("auth-task-done")
    Then a checkpoint is saved with the name "auth-task-done"
    And the agent receives confirmation

  Scenario: Agent creates a timestamped checkpoint
    Given an agent transitioning between tasks
    When the agent calls checkpoint() with no name
    Then a checkpoint is created with a generated timestamp-based name
    And the agent receives the generated name

  Scenario: Checkpoint signals task boundary
    Given an agent calling checkpoint() before starting a new task
    When the new task begins
    Then the agent starts from a clean context prescription
    And does not carry assumptions from the previous task
```

### recommend_next

Given a task description, the MCP server prescribes the minimal context the agent should load.

```gherkin
Feature: recommend_next

  Scenario: Discovery task gets L0 prescription
    Given an agent with the task "list all functions in the billing module"
    When the agent calls recommend_next("list all functions in the billing module")
    Then the prescription specifies list_exports("src/billing") at L0
    And does not suggest loading full source files

  Scenario: Understanding task gets L1/L2 prescription
    Given an agent with the task "explain what createInvoice does"
    When the agent calls recommend_next("explain what createInvoice does")
    Then the prescription specifies get_file_context at L1 for createInvoice
    And suggests load_context("orientation") if not already loaded

  Scenario: Edit task gets L3 prescription for target, L0 for context
    Given an agent with the task "fix the rounding bug in calculateTax"
    When the agent calls recommend_next("fix the rounding bug in calculateTax")
    Then the prescription specifies L3 for the file containing calculateTax
    And L0 for adjacent files that may be relevant

  Scenario: General task gets orientation prescription
    Given an agent with an unclear or broad task
    When the agent calls recommend_next(task)
    Then the prescription defaults to get_llmspec() for orientation
    And suggests narrowing after orientation
```

## Status

| Feature | Status |
|---------|--------|
| Targeted slice loading (load_context) | 🔲 Planned |
| Token budget reporting per slice | 🔲 Planned |
| Context summary (get_context_summary) | 🔲 Planned |
| Named checkpoint (in-session task boundary) | 🔲 Planned |
| Timestamped checkpoint | 🔲 Planned |
| recommend_next (task-to-context prescription) | 🔲 Planned |
| Session protocol documentation | 🔲 Planned |

> **Cross-session context management** (session resume, decision capture, pattern capture, open items) is covered in [08-ProjectWorkflow](./08-ProjectWorkflow.md).
