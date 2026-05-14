---
id: tdd-testability
type: feature
traces: ideas/18-testability-tdd.md
---

# TDD & Testability Guidance

> Sensei ensures the AI writes testable code — decomposed, test-first, with human approval of test cases before implementation.

AI-generated code tends toward monoliths. Tests written after implementation just validate what was built, not what should be built. Sensei's build command enforces decomposition, presents tests for approval, and scores testability from function shape.

## Features

### Test-First with Human Approval

```gherkin
Feature: Test-First Workflow

  Scenario: AI presents tests before implementation
    Given a task to implement a new function
    When /sensei:build runs
    Then the AI writes test cases first
    And presents them to the user: "Here are the test cases. Do they cover the right behavior?"
    And waits for approval before implementing

  Scenario: User adjusts test cases
    Given the AI presented 4 test cases
    When the user says "also test the empty input case"
    Then the AI adds the test case
    And re-presents for approval

  Scenario: Tests drive implementation
    Given approved test cases
    When the AI implements the function
    Then all approved tests pass
    And no tests are removed or weakened to make implementation easier
```

### Function Decomposition

```gherkin
Feature: Function Decomposition

  Scenario: AI proposes decomposition before coding
    Given a task that involves multiple operations
    When /sensei:build runs
    Then the AI proposes: "I'll create 3 functions:
      - extract_symbols(ast) → Vec<Symbol> (pure, unit testable)
      - write_symbols(symbols, graph) (side-effect, integration testable)
      - process_file(path, graph) (orchestrator, calls both)"
    And waits for user confirmation

  Scenario: AI avoids monolith
    Given a task that could be a single large function
    When /sensei:build runs
    Then the AI separates pure logic from side effects
    And each pure function has 0-3 parameters
    And side effects are isolated in thin orchestrator functions
```

### Testability Scoring

```gherkin
Feature: Testability Scoring

  Scenario: High testability function
    Given a function with 2 params, no side effects, complexity 3
    When testability is assessed
    Then the score is high
    And no recommendations are made

  Scenario: Low testability function
    Given a function with 5 params, 3 side effects, complexity 15
    When testability is assessed
    Then the score is low
    And recommendations include: "split into pure logic + orchestrator", "reduce params"
```

## Status

| Feature | Status |
|---------|--------|
| Test-first with approval | Planned |
| Function decomposition guidance | Planned |
| Testability scoring | Planned |
| Decomposition logging (events) | Planned |
