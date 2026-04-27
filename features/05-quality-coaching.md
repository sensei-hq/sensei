---
id: quality-coaching
type: feature
traces: ideas/07-metrics-analytics.md, ideas/19-benchmarking-credibility.md
---

# Quality Metrics & Coaching

> Sensei measures development quality and provides guided advice — so you can see what's improving and what needs attention.

Without measurement, you can't tell if AI-assisted development is getting better. Sensei captures events, computes metrics, shows trends on the desktop dashboard, and generates actionable prompts when quality drops.

## Features

### Automatic Event Capture

```gherkin
Feature: Event Capture

  Scenario: Tool usage captured automatically
    Given a coding session with sensei active
    When the AI calls any tool
    Then pre-tool and post-tool hooks log the event to the daemon
    And each event includes: tool name, whether it's an MCP tool, phase context

  Scenario: Turns counted automatically
    Given a coding session
    When the user sends a message
    Then the user-prompt hook increments the turn count for the active task
    And classifies the prompt as correction/continuation/clarification/new_request

  Scenario: Corrections detected for FTR
    Given the user says "no, that's wrong — use the adapter pattern"
    When the user-prompt hook classifies the prompt
    Then it detects a correction
    And fires a revision_requested event
    And the active task's FTR is marked as not-first-try
```

### Metrics Dashboard

```gherkin
Feature: Quality Dashboard

  Scenario: FTR trend visible
    Given 20 completed tasks over the past 2 weeks
    When the developer opens the desktop quality dashboard
    Then FTR trend is displayed as a line chart
    And current FTR percentage is shown

  Scenario: Drill into rework
    Given FTR dropped from 80% to 60% this week
    When the developer clicks the FTR metric
    Then it shows which tasks had rework
    And which corrections were made
    And which modules are most affected
```

### Guided Coaching

```gherkin
Feature: Guided Coaching

  Scenario: System generates actionable recommendation
    Given pattern adherence dropped for the adapters module
    When the developer views the coaching panel
    Then it shows: "Pattern adherence dropped. 3 corrections this week: missing adapter trait.
      Recommended: add guardrail 'all parsers must implement LanguageAdapter'"
    And provides an "Add guardrail" button

  Scenario: In-session coaching
    Given the AI is building an api route
    And historical data shows api routes have 40% rework from missing validation
    When the AI starts the locate step
    Then it proactively says: "Your api routes sometimes miss validation middleware.
      Should I include it? (Based on 3 recent corrections)"
```

## Status

| Feature | Status |
|---------|--------|
| Event capture (hooks) | Planned (hooks exist, need wiring) |
| Turn counting + correction detection | Planned |
| Metrics computation (daemon) | Planned |
| Quality dashboard (desktop) | Planned |
| Guided coaching | Planned |
| Benchmarking | Planned |
