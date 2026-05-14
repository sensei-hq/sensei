---
id: rules-context
type: feature
traces: ideas/04-cross-cutting.md, blueprints/01-workflow-engine.md
---

# Rules & Context Preservation

> Sensei remembers the rules even when the AI forgets — rules persist across sessions and survive context compaction.

In long conversations, the AI forgets constraints. It ignores patterns established 50 turns ago, uses grep instead of the graph, and makes mistakes it was corrected for in a previous session. Sensei's rules are a living document that grows from corrections, loaded automatically every session, and preserved across compaction.

## Features

### Rules as a Living Document

```gherkin
Feature: Rules

  Scenario: Rules loaded at session start
    Given a project with .sensei/rules.md containing 5 rules
    When a new Claude Code session starts
    Then the session-start hook injects a rules summary into context
    And the AI can reference the rules when making decisions

  Scenario: Rules grow from corrections
    Given the AI wrote code that doesn't use the adapter pattern
    And the user corrects: "you should have used the adapter pattern"
    When the user types /sensei:rules
    Then the AI asks clarifying questions about when to use the pattern
    And adds the rule to .sensei/rules.md
    And next session loads the updated rules automatically

  Scenario: Rules survive compaction
    Given rules are loaded and the conversation is long
    When Claude Code compacts the context
    Then the pre-compact hook preserves rules in the compressed context
    And the AI can still reference the rules after compaction
```

### Refocus Commands

```gherkin
Feature: Refocus

  Scenario: Manual refocus after drift
    Given the AI has drifted from the current task
    When the user types /sensei:session refocus
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
    When the user types /sensei:session status
    Then the AI displays: phase, plan, task, issue, rules count, pattern count, doc counts, tool status
```

## Status

| Feature | Status |
|---------|--------|
| Rules file | Planned |
| Session-start rules injection | Planned |
| Pre-compact hook | Planned |
| /sensei:rules command | Planned |
| /sensei:session refocus command | Planned |
| /sensei:tools command | Planned |
| /sensei:session status command | Planned |
