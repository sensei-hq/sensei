# Content Compression

Loading full source files into LLM context is the single biggest source of token waste in agentic coding. Most tasks — understanding a function, tracing a flow, discovering what's available — need far less than full source. Content compression defines four resolution levels and the rules for choosing between them.

## Features

### Resolution Levels

Code and documentation are stored and served at four levels of detail. Agents request the level they need; the MCP server delivers it.

```gherkin
Feature: Resolution Levels

  Scenario: Agent discovers available functions at L0
    Given an indexed module with five exported functions
    When the agent calls get_file_context("src/orders.ts", "L0")
    Then five function signatures are returned
    And each signature includes parameter names, types, and return type
    And no function body or implementation is included
    And total tokens are under 100

  Scenario: Agent understands what a function does at L1
    Given an indexed function processOrder(orderId)
    When the agent calls get_file_context("src/orders.ts", "L1") for processOrder
    Then the response shows: order = processOrder(orderId)
    And the input type (string) and output type (Promise<Order>) are shown
    And the implementation is not included

  Scenario: Agent traces a flow at L2
    Given a function with five logical steps
    When the agent calls get_file_context("src/orders.ts", "L2") for processOrder
    Then the response lists the steps in plain English
    And uses pipeline notation where appropriate: validate → fetch → charge → emit
    And no code syntax is included

  Scenario: Agent edits code at L3
    Given a function the agent needs to modify
    When the agent calls get_file_context("src/orders.ts", "L3")
    Then the full source of the file is returned
    And the agent can make precise edits with full context
```

### Docstring and Comment Stripping

Docstrings and inline doc-comments duplicate what signatures already communicate to LLMs. They are stripped at all levels below L3.

```gherkin
Feature: Docstring Stripping

  Scenario: JSDoc comments are excluded below L3
    Given a TypeScript file where every function has a JSDoc block
    When the agent retrieves the file at L0, L1, or L2
    Then the JSDoc blocks are not present in the response
    And the function signatures and logic are intact

  Scenario: Inline explanatory comments are excluded at L0 and L1
    Given a function with helpful inline comments explaining the logic
    When the agent retrieves at L0 or L1
    Then inline comments are excluded
    And the signature or IO pattern is still complete

  Scenario: Full source at L3 includes all comments
    Given a file with docstrings and inline comments
    When the agent retrieves at L3
    Then all comments are present exactly as written
```

### Logic Flow Notation

At L2, code is expressed as structured plain-English logic, not syntax. This allows agents to reason about behaviour without parsing language-specific constructs.

```gherkin
Feature: Logic Flow Notation

  Scenario: If/else expressed as indented bullets
    Given a function with an if/else block
    When the agent retrieves at L2
    Then the condition is expressed as: "if valid → proceed"
    And the else branch as: "else → throw ValidationError"
    And no if/else syntax is present

  Scenario: Pipeline expressed with arrows
    Given a function that calls parse, validate, then transform in sequence
    When the agent retrieves at L2
    Then the flow is: raw → parse() → validate() → transform() → output
    And each step is one token-efficient term

  Scenario: Async flow expressed naturally
    Given an async function that fetches, checks, and returns data
    When the agent retrieves at L2
    Then the flow is: await fetch → check status → parse body → return
    And no async/await syntax is present

  Scenario: State machine expressed as transitions
    Given a stateful component with idle, loading, success, and error states
    When the agent retrieves at L2
    Then the states are expressed as: idle → loading → success / error
    And transition conditions are noted inline where relevant
```

### IO Pattern Notation

At L1, functions are expressed as their most useful form: what goes in, what comes out.

```gherkin
Feature: IO Pattern Notation

  Scenario: Function IO expressed as assignment pattern
    Given a function processOrder(orderId: string): Promise<Order>
    When the agent retrieves at L1
    Then the response is: order = processOrder(orderId)
    And the input type (string) is shown
    And the output type (Promise<Order>) is shown
    And no implementation is included

  Scenario: Multi-parameter function IO
    Given a function with three parameters
    When the agent retrieves at L1
    Then all parameters are shown in the assignment pattern
    And each parameter type is noted

  Scenario: Void function IO
    Given a function that returns void
    When the agent retrieves at L1
    Then the response indicates no return value
    And side effects are noted if detectable from the signature
```

### Task-to-Level Mapping

The correct resolution level is determined by the task, not by the agent's preference. MCP tools expose this mapping so agents don't have to decide.

```gherkin
Feature: Task-to-Level Mapping

  Scenario: Discovery task maps to L0
    Given an agent asked to "list available functions in the auth module"
    When the agent calls recommend_next("list available functions in auth module")
    Then the recommendation specifies L0 for the auth module
    And no higher resolution is suggested

  Scenario: Understanding task maps to L1 or L2
    Given an agent asked to "explain what processOrder does"
    When the agent calls recommend_next("explain what processOrder does")
    Then the recommendation specifies L1 for processOrder

  Scenario: Debugging task maps to L2 or L3
    Given an agent asked to "trace the order processing flow"
    When the agent calls recommend_next("trace order processing flow")
    Then the recommendation specifies L2 for the relevant module

  Scenario: Edit task maps to L3
    Given an agent asked to "fix the bug in validateToken"
    When the agent calls recommend_next("fix the bug in validateToken")
    Then the recommendation specifies L3 for the file containing validateToken
    And suggests L0 for adjacent files for context
```

## Status

| Feature | Status |
|---------|--------|
| L0 — Signature level | 🔲 Planned |
| L1 — IO pattern level | 🔲 Planned |
| L2 — Logic flow level | 🔲 Planned |
| L3 — Full source level | 🔲 Planned |
| Docstring stripping at L0/L1/L2 | 🔲 Planned |
| Logic flow notation (L2) | 🔲 Planned |
| IO pattern notation (L1) | 🔲 Planned |
| Task-to-level mapping via recommend_next | 🔲 Planned |
| Import stripping at L0/L1/L2 | 🔲 Planned |
