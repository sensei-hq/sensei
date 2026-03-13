---
id: skill-tooling
type: feature
---

# Skill Tooling

> Agents can create, test, install, and improve skills — teachable behaviour modules that persist across every future session

Skills are reusable behaviour modules that agents load to follow a consistent protocol, apply a technique, or access reference knowledge. Without a disciplined workflow for creating and validating skills, agents produce skills that fail under real working conditions: edge cases the author didn't anticipate, pressure scenarios that the skill does not survive, token budgets that don't reflect actual use. Skill Tooling closes that gap by giving agents a repeatable, test-driven workflow for producing skills that work.

## Features

### Skill Creation Workflow

Agents use a test-driven process to create new skills. The process moves through four phases — baseline, write, pressure-test, and refactor — and produces a skill that has been validated against realistic pressure before it is committed. No skill is considered complete until it has survived the pressure phase.

```gherkin
Feature: Skill Creation Workflow

  Scenario: Agent runs a baseline test before writing a skill
    Given an agent is asked to create a skill for a specific protocol
    When the agent runs the baseline phase
    Then the agent attempts the target task without any skill loaded
    And the agent records every failure, shortcut, and rationalisation
    And the baseline results are preserved as the benchmark for the finished skill

  Scenario: Agent writes a skill to address documented failures
    Given a completed baseline showing the agent violating the target protocol
    When the agent writes the skill
    Then the skill addresses each failure mode identified in the baseline
    And the skill is structured to be readable and actionable under pressure
    And the skill stays within the token budget for its type

  Scenario: Agent pressure-tests the skill before declaring it done
    Given a written skill
    When the agent runs the pressure test phase
    Then the agent attempts the target task under combined pressure scenarios
    And each scenario combines at least three simultaneous pressures (e.g., time pressure, sunk cost, authority pressure)
    And the agent records whether the skill was cited and the protocol was followed
    And only a skill that produces consistent compliance across all scenarios passes

  Scenario: Agent refactors the skill based on pressure-test findings
    Given a pressure test that revealed compliance failures
    When the agent refactors the skill
    Then the skill is rewritten to address the specific pressure scenarios that caused failure
    And the pressure test is re-run after each refactor
    And the process continues until the skill achieves the stopping condition

  Scenario: Stopping condition is defined and enforced
    Given a skill under pressure testing
    When the agent reaches the stopping condition
    Then the agent has complied with the skill protocol and cited the skill under maximum pressure in the final run
    And no further refactor is needed
    And the skill is ready for installation
```

---

### Skill Discovery

Agents can discover which skills are available and invoke them by name. Discovery is passive — agents can inspect the available skills and their triggering conditions to decide which to load for the current task.

```gherkin
Feature: Skill Discovery

  Scenario: Agent lists available skills
    Given one or more skills are installed
    When the agent lists available skills
    Then the agent receives a list of skill names with their triggering descriptions
    And the list reflects the currently installed skills at the time of the call

  Scenario: Agent selects the right skill by matching triggering conditions
    Given an agent beginning a task that matches the conditions of an installed skill
    When the agent reviews the skill descriptions
    Then the agent identifies the matching skill
    And loads it before proceeding with the task

  Scenario: Agent invokes a skill by name
    Given an installed skill with a known name
    When the agent invokes the skill
    Then the full skill content is loaded into the agent's context
    And the agent proceeds according to the skill protocol

  Scenario: No skill matches the current task
    Given a task with no matching installed skill
    When the agent reviews available skills
    Then no match is found
    And the agent proceeds without loading a skill
    And no error is raised
```

---

### Skill Installation

Skills developed in a project can be installed globally so they are available in every future session across all projects. Installation is the promotion step that takes a project-local skill and makes it globally accessible.

```gherkin
Feature: Skill Installation

  Scenario: Agent installs a project skill globally
    Given a skill has been created and validated in the current project
    When the agent installs the skill
    Then the skill becomes available globally
    And subsequent sessions in any project can discover and invoke the skill

  Scenario: Installing a skill does not modify the project source
    Given a validated skill in the project
    When the agent installs the skill globally
    Then the project source files are unchanged
    And the globally installed copy reflects the current state of the project skill

  Scenario: Installing an updated skill replaces the previous global version
    Given a skill that has been updated since its last installation
    When the agent re-installs the skill
    Then the globally installed copy is replaced with the updated version
    And the previous version is no longer loaded by new sessions

  Scenario: Global installation persists across machine restarts
    Given a skill that has been installed globally
    When the agent starts a new session in a different project
    Then the skill is still available for discovery and invocation
    And no re-installation is required
```

---

### Skill Lifecycle

Skills are living artefacts. After initial installation, they can be updated in response to new failure modes, improved based on real-world use, re-tested after changes, and retired when no longer needed.

```gherkin
Feature: Skill Lifecycle

  Scenario: Skill is improved after a failure observed in production use
    Given an installed skill that failed to prevent a protocol violation in a real session
    When the agent updates the skill to address the new failure mode
    And re-runs the pressure test suite
    Then the updated skill passes the pressure tests including the new scenario
    And the updated version is re-installed globally

  Scenario: Skill is re-tested after any content change
    Given a skill whose content has been modified
    When the agent runs the pressure test
    Then the full pressure suite is run against the updated content
    And the results are recorded to confirm no regression

  Scenario: Skill is retired when it is no longer applicable
    Given a skill that covers a workflow or tool that no longer applies
    When the skill is retired
    Then the skill is removed from the global installation
    And it is no longer surfaced during skill discovery
    And the project source is preserved for reference

  Scenario: Skill version history is preserved in the project repo
    Given a skill that has been through multiple refactor cycles
    When the agent reviews the skill history
    Then each revision is available through the project's version control history
    And the rationale for each change is recorded in commit messages
```

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| reliability | A skill that passes the pressure test phase must continue to produce compliant agent behaviour in real sessions |
| token efficiency | Skills must stay within type-appropriate token budgets so they remain loadable without consuming excessive context |
| portability | Globally installed skills must be available in any project without reconfiguration |
| testability | Every skill must have a documented set of pressure scenarios that can be re-run after any change |
