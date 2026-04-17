---
id: workflow-commands
type: feature
traces: ideas/01-workflow-system.md, ideas/02-commands.md
---

# Workflow Commands

> Sensei guides your AI assistant through structured development phases — from idea to implementation — so it builds the right thing, the right way, on the first try.

AI coding assistants jump straight to code. They skip design, ignore patterns, and produce throwaway work when requirements are unclear. Sensei's workflow commands set intent — each command tells the AI what phase it's in, what prior work to read, what constraints apply, and what to produce. The human stays in control; the AI stays focused.

## Features

### Phase Commands

The user invokes a phase command to set the AI's mode. Each command loads relevant context and constrains behavior.

```gherkin
Feature: Phase Commands

  Scenario: User starts ideation
    Given a project with sensei configured
    When the user types /sensei:idea "task scheduler"
    Then the AI asks clarifying questions about the problem
    And produces a doc in docs/ideas/ using the idea template
    And does NOT write any code
    And updates .sensei/state.yaml with active_phase: ideate

  Scenario: User moves to analysis
    Given an idea doc exists at docs/ideas/task-scheduler.md
    When the user types /sensei:analyze
    Then the AI reads the idea doc
    And scans the codebase for related patterns and existing code
    And produces a doc in docs/analysis/ with 2-3 approaches and tradeoffs
    And does NOT write any code

  Scenario: User starts building
    Given a plan exists with GitHub issues
    When the user types /sensei:build
    Then the AI picks the highest priority open issue
    And runs the locate step (search, get_patterns, get_callers via MCP)
    And proposes a function decomposition before coding
    And writes tests before implementation
    And presents tests to the user for approval
    And updates state.yaml with active_task and active_issue

  Scenario: Phase transition nudge
    Given the AI is in ideation phase
    And the conversation has produced sufficient detail for analysis
    When the AI detects the depth is beyond ideation
    Then it suggests: "This is deep enough for ideation — ready for /sensei:analyze?"
    And does NOT auto-advance without user confirmation
```

### Brainstorm Command

The primary creative command. One conversation produces artifacts at multiple depth levels.

```gherkin
Feature: Brainstorm

  Scenario: Content routes to natural depth
    Given the user types /sensei:brainstorm
    When the conversation produces idea-level content
    Then it goes to docs/ideas/
    When the conversation produces design-level content
    Then it goes to docs/blueprints/
    And each artifact has frontmatter with origin tracing

  Scenario: AI asks before routing
    Given a brainstorm conversation
    When the AI detects content is getting into blueprint territory
    Then it asks: "This is getting into architecture — should I put this in docs/blueprints/?"
    And waits for user confirmation before writing
```

### Plan and Backlog Integration

Plans decompose into GitHub issues for tracking.

```gherkin
Feature: Plan and Backlog

  Scenario: Plan creates GitHub issues
    Given an approved blueprint at docs/blueprints/workflow-engine.md
    When the user types /sensei:plan
    Then the AI decomposes the blueprint into ordered features
    And creates a GitHub issue for each feature with acceptance criteria
    And applies labels: concept, depth, wave, priority, type
    And records the plan in docs/plans/

  Scenario: Build picks next issue
    Given open GitHub issues with wave:1 label
    When the user types /sensei:build
    Then the AI picks the highest priority open issue
    And sets active_issue in state.yaml
    And includes "Closes #N" in the commit message on completion
```

## Status

| Feature | Status |
|---------|--------|
| Phase commands (7) | Planned |
| Brainstorm command | Planned |
| Plan + GitHub integration | Planned |
| Phase transition nudges | Planned |
