---
id: pattern-enforcement
type: feature
traces: ideas/15-pattern-store.md, ideas/17-pattern-knowledge.md
---

# Pattern Detection & Enforcement

> Sensei detects your codebase patterns and ensures the AI follows them — before and after code is written.

AI assistants reinvent structure that already exists. Sensei detects patterns from code (adapters, factories, workers), from library conventions (rokkit component structure), and from industry best practices (patterns.dev). During coding, it presents applicable patterns. During review, it catches violations.

## Features

### Pattern Detection

```gherkin
Feature: Pattern Detection

  Scenario: Detect patterns by naming convention
    Given a project with TypeScriptAdapter, PythonAdapter, RustAdapter, JavaAdapter
    When the indexer runs
    Then a pattern named "language-adapter" is detected with 4 instances
    And the pattern includes the shared interface and file locations

  Scenario: Detect patterns structurally
    Given a class implements LanguageAdapter trait and wraps a parser
    When the indexer runs with structural heuristics enabled
    Then the class is identified as an adapter pattern instance
    And the trait, wrapping relationship, and registration point are recorded
```

### Pattern Enforcement During Build

```gherkin
Feature: Pattern Enforcement in Build

  Scenario: AI finds applicable pattern before coding
    Given a task: "add SQL parsing support"
    And a detected pattern: "language-adapter" with 4 instances
    When the AI runs the locate step in /sensei:build
    Then it calls match_pattern("SQL parsing")
    And presents: "Found pattern: language-adapter. Following TypeScriptAdapter as reference."
    And follows the pattern structure

  Scenario: AI asks when unsure about pattern
    Given a task that partially matches an existing pattern
    When the AI runs the locate step
    Then it asks: "I found a possible pattern match (adapter). Should I follow it?"
    And waits for user confirmation

  Scenario: No matching pattern
    Given a task with no matching patterns
    When the AI runs the locate step
    Then it proceeds without pattern constraint
    And still decomposes and writes tests first
```

### Pattern Enforcement During Review

```gherkin
Feature: Pattern Enforcement in Review

  Scenario: Review catches pattern violation
    Given new code that looks like an adapter but doesn't implement the trait
    When /sensei:review runs
    Then it flags: "sql.rs doesn't implement LanguageAdapter — should it be an adapter?"

  Scenario: Review catches new duplication
    Given new code that duplicates logic from an existing file
    When /sensei:review runs
    Then it flags: "parse_sql() duplicates logic from parse_json() — consider shared helper"
```

### Pattern Options

```gherkin
Feature: Pattern Options

  Scenario: AI presents architectural options on first encounter
    Given a new project with no established data loading pattern
    When the AI needs to load data for a page
    Then it presents options: SSR loader, client-side fetch, reactive streams
    And includes tradeoffs for each
    And waits for user to pick

  Scenario: AI follows established convention
    Given all existing pages use SSR loader pattern
    When the AI builds a new page
    Then it follows SSR loader without asking
    And mentions: "Following your established SSR loader pattern"
```

## Status

| Feature | Status |
|---------|--------|
| Pattern detection Phase A (naming) | Planned |
| Pattern detection Phase B (structural) | Planned |
| Locate step pattern check | Planned |
| Review pattern enforcement | Planned |
| Pattern options presentation | Planned |
| Library pattern extraction | Planned |
