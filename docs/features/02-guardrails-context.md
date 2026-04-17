---
id: guardrails-context
type: feature
traces: ideas/04-cross-cutting.md, blueprints/01-workflow-engine.md
---

# Guardrails & Context Preservation

> Sensei remembers the rules even when the AI forgets — guardrails persist across sessions and survive context compaction.

In long conversations, the AI forgets constraints. It ignores patterns established 50 turns ago, uses grep instead of the graph, and makes mistakes it was corrected for in a previous session. Sensei's guardrails are a living document that grows from corrections, loaded automatically every session, and preserved across compaction.

## Features

### Guardrails as a Living Document

```gherkin
Feature: Guardrails

  Scenario: Guardrails loaded at session start
    Given a project with .sensei/guardrails.md containing 5 rules
    When a new Claude Code session starts
    Then the session-start hook injects a guardrails summary into context
    And the AI can reference the rules when making decisions

  Scenario: Guardrails grow from corrections
    Given the AI wrote code that doesn't use the adapter pattern
    And the user corrects: "you should have used the adapter pattern"
    When the user types /sensei:guardrails
    Then the AI asks clarifying questions about when to use the pattern
    And adds the rule to .sensei/guardrails.md
    And next session loads the updated guardrails automatically

  Scenario: Guardrails survive compaction
    Given guardrails are loaded and the conversation is long
    When Claude Code compacts the context
    Then the pre-compact hook preserves guardrails in the compressed context
    And the AI can still reference the rules after compaction
```

### Refocus Commands

```gherkin
Feature: Refocus

  Scenario: Manual refocus after drift
    Given the AI has drifted from the current task
    When the user types /sensei:refocus
    Then the AI re-reads state.yaml, the active plan, and current task
    And outputs: current phase, task, issue, and what's left
    And returns to focused work on the correct task

  Scenario: Tool awareness reload
    Given the AI is using grep instead of sensei MCP search
    When the user types /sensei:tools
    Then the AI re-reads available MCP tools and their purposes
    And acknowledges: "I should use search() instead of grep for symbol lookup"

  Scenario: Status check
    Given a session in progress
    When the user types /sensei:status
    Then the AI displays: phase, plan, task, issue, guardrails count, pattern count, doc counts, tool status
```

## Status

| Feature | Status |
|---------|--------|
| Guardrails file | Planned |
| Session-start guardrails injection | Planned |
| Pre-compact hook | Planned |
| /sensei:guardrails command | Planned |
| /sensei:refocus command | Planned |
| /sensei:tools command | Planned |
| /sensei:status command | Planned |
